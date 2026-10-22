//! Visible light cones of street lanterns and vehicle headlights.
//!
//! Design and measurements: `docs/src/architecture/lighting-cones.md`. A cone is an entity with a
//! shared unit cone mesh (apex at the origin, axis -Z), scaled to the cone's length and base
//! radius, drawn by [`LightConeMaterial`]: back faces only, a ray-marched additive scatter inside
//! the analytic cone (8 steps near, 4 far), soft radial and axial falloff, and the camera may
//! be inside. The per-cone colour, brightness and inner edge ride in the entity's [`MeshTag`]
//! ([`pack_tag`]), so all cones are one instanced draw.
//!
//! * **Sources** (data only, no Bevy lights): a scenery `[light_enh_2]` with the `cone` flag
//!   ([`lantern_cone_shape`]), a vehicle `[light_enh_2]` headlight ([`headlight_cone_shape`]) and
//!   the player vehicle's `[spotlight]` ([`spotlight_cone_shape`]). The brightness is *read* from
//!   the existing light plumbing ([`ConeLevel`]); nothing is evaluated twice.
//! * **Budget** ([`ConePool`], [`light_cone_pool_system`]): of all lit cones within range, only
//!   the [`LightQuality::cone_cap`] with the largest screen coverage are visible (with
//!   hysteresis); every other cone is `Hidden` and costs nothing.
//! * **Depth fade** ([`cone_depth_prepass_system`]): while [`LightQuality::cone_depth_fade`] is
//!   on and cones shine, the main camera gets a [`DepthPrepass`]; the shader then ends each
//!   cone's volume at the scene geometry.
//!
//! [`LightConePlugin`] holds the logic (headless-safe); [`LightConeRenderPlugin`] adds the
//! material, the shader and the mesh (needs the render app) and is added by the game.

use bevy::asset::{load_internal_asset, RenderAssetUsages};
use bevy::core_pipeline::prepass::DepthPrepass;
use bevy::light::{NotShadowCaster, NotShadowReceiver};
use bevy::mesh::MeshTag;
use bevy::mesh::{Indices, MeshVertexBufferLayoutRef, PrimitiveTopology, VertexAttributeValues};
use bevy::pbr::{Material, MaterialPipeline, MaterialPipelineKey, MaterialPlugin};
use bevy::prelude::*;
use bevy::reflect::TypePath;
use bevy::render::render_resource::{
    AsBindGroup, BlendComponent, BlendFactor, BlendOperation, BlendState, Face,
    RenderPipelineDescriptor, SpecializedMeshPipelineError,
};
use bevy::shader::ShaderRef;
use opts_core::formats::scenery::ScoLight;
use opts_core::formats::vehicles::model::Spotlight;

use crate::coords::omsi_to_bevy_pos;
use crate::light_quality::{LightQuality, LightQualityPlugin, LightQualitySet, MAX_CONE_CAP};
use crate::scenery_runtime::{SceneryLight, SceneryRuntimeSet};
use crate::vehicle_lights::ScriptLight;

// ---------------------------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------------------------

/// Lantern cones: the half angle is the light's outer angle halved, kept in this range (degrees).
/// The `[light_enh_2]` angles describe where the corona is visible and reach 200 degrees; a beam
/// of that width is no cone.
pub const LANTERN_HALF_DEG: (f32, f32) = (10.0, 40.0);
/// Length of a lantern cone that does not point at the ground (m).
pub const LANTERN_DEFAULT_LENGTH_M: f32 = 6.0;
/// Shortest and longest lantern cone (m); the length is the distance to the ground when known.
pub const LANTERN_LENGTH_M: (f32, f32) = (2.0, 14.0);
/// Vehicle headlight cones: half-angle range (degrees).
pub const HEADLIGHT_HALF_DEG: (f32, f32) = (6.0, 25.0);
/// Half angle of the haze cone of a headlight whose outer angle is a corona angle or whose spot is wide
/// (above 90 degrees), in degrees.
pub const HEADLIGHT_CORONA_BEAM_HALF_DEG: f32 = 11.0;
/// Length of a headlight cone built from a `[light_enh_2]` (it has no range) (m).
pub const HEADLIGHT_CONE_LENGTH_M: f32 = 30.0;
/// Longest cone of a `[spotlight]` (its range is 100 to 500 m) (m).
pub const SPOT_CONE_MAX_LENGTH_M: f32 = 40.0;
/// Lantern cones farther than this from the camera are not considered (m).
pub const LANTERN_MAX_DISTANCE_M: f32 = 150.0;
/// Headlight cones farther than this from the camera are not considered (m).
pub const HEADLIGHT_MAX_DISTANCE_M: f32 = 250.0;
/// A cone is lit above this brightness level.
pub const CONE_ON_LEVEL: f32 = 0.02;
/// A cone that holds a slot keeps it while its coverage times this beats the newcomers'.
pub const CONE_HYSTERESIS: f32 = 1.3;
/// Seconds between two selections of the visible cones (a change of the quality settings
/// selects at once).
pub const SELECT_INTERVAL_S: f32 = 0.15;
/// Seconds a camera must have had the depth prepass before cones are drawn with the depth fade
/// on. The pipeline variants with the prepass compile in the background; until they exist the
/// cones would draw without depth (a white-out). Measured live: a few seconds right after the
/// switch, so the gate is conservative.
pub const PREPASS_READY_S: f32 = 3.0;
/// Frames the camera must have had the prepass (besides the time).
pub const PREPASS_READY_FRAMES: u32 = 6;
/// Seconds the cones take to fade in once the prepass counts as ready.
pub const CONE_FADE_IN_S: f32 = 0.3;
/// Seconds the depth prepass stays on after the last cone went out (so the prepass pipelines are
/// not rebuilt every time a headlight is switched off).
pub const PREPASS_RELEASE_S: f32 = 20.0;

// ---------------------------------------------------------------------------------------------
// Pure mapping
// ---------------------------------------------------------------------------------------------

mod shape;
pub use shape::{
    cone_angles, headlight_cone_shape, lantern_cone_length, lantern_cone_shape,
    spotlight_cone_shape, ConeKind, ConeShape,
};

/// The ground clip height for an apex `y` metres above the object's origin (which stands on the
/// ground); none when the apex is not clearly above it.
fn ground_height(y: f32) -> Option<f32> {
    (y > 0.3).then_some(y.min(GROUND_H_MAX_M))
}

/// Highest ground height a tag can carry (m); the field has 0.25 m steps.
pub const GROUND_H_MAX_M: f32 = 31.5;

/// Packs the per-cone shader data into the mesh tag: colour (5 bits per channel, bits 0..14),
/// brightness (6 bits, 15..20), inner edge (4 bits, 21..24) and the apex height above the
/// ground in 0.25 m steps (7 bits, 25..31; 127 = no ground clip).
#[must_use]
pub fn pack_tag(color: [f32; 3], level: f32, inner_frac: f32, ground_h: Option<f32>) -> u32 {
    let c = |v: f32| (v.clamp(0.0, 1.0) * 31.0).round() as u32;
    let i = (level.clamp(0.0, 1.0) * 63.0).round() as u32;
    let e = (inner_frac.clamp(0.0, 1.0) * 15.0).round() as u32;
    let g = ground_h.map_or(127, |h| (h.clamp(0.0, GROUND_H_MAX_M) * 4.0).round() as u32);
    c(color[0]) | (c(color[1]) << 5) | (c(color[2]) << 10) | (i << 15) | (e << 21) | (g << 25)
}

/// The brightness field of a tag (`0..=63`).
#[must_use]
pub const fn tag_level(tag: u32) -> u32 {
    (tag >> 15) & 63
}

/// The ground height field of a tag in metres, if it has one.
#[must_use]
pub fn tag_ground_h(tag: u32) -> Option<f32> {
    let g = tag >> 25;
    (g != 127).then_some(g as f32 * 0.25)
}

/// Where a cone's brightness comes from. The cone only *reads* a level that the existing light
/// plumbing computes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConeLevel {
    /// The `level` of the [`SceneryLight`] on this entity (scenery lights and vehicle
    /// `[light_enh_2]` glows: variable x factor, time constant, `NightlightA`, `AI_Light`).
    Glow(Entity),
    /// The `SpotLight` of this entity: its intensity over the [`ScriptLight`]'s base intensity
    /// (`Spot_Select`).
    Spot(Entity),
    /// A fixed level (tests, tools).
    Fixed(f32),
}

/// A visible light cone. The entity is a child of the object (or lamp) it belongs to.
#[derive(Component, Debug, Clone)]
pub struct LightCone {
    /// Which setting gates it.
    pub kind: ConeKind,
    /// Geometry and colour.
    pub shape: ConeShape,
    /// Where the brightness comes from.
    pub level: ConeLevel,
}

/// Spawns a hidden cone entity under `parent`. The mesh and material are added by
/// [`LightConeRenderPlugin`].
pub fn spawn_light_cone(commands: &mut Commands, parent: Entity, cone: LightCone) -> Entity {
    let transform = cone.shape.transform();
    let e = commands
        .spawn((
            transform,
            Visibility::Hidden,
            MeshTag::new(0),
            NotShadowCaster,
            NotShadowReceiver,
            cone,
            Name::new("LightCone"),
        ))
        .id();
    commands.entity(parent).add_child(e);
    e
}

// ---------------------------------------------------------------------------------------------
// Selection
// ---------------------------------------------------------------------------------------------

/// Screen coverage of a cone of `length` and base `radius` whose centre is `distance` away: its
/// projected area up to a constant, `length * radius / distance^2` (distances below 1 m count as
/// 1 m).
#[must_use]
pub fn cone_coverage(length: f32, radius: f32, distance: f32) -> f32 {
    length * radius / (distance * distance).max(1.0)
}

/// The cones that should be visible: the `cap` of `candidates` (cone, coverage) with the
/// largest coverage, where a cone in `current` counts `hysteresis` times its coverage. Ties
/// break by entity, so the choice is stable from one selection to the next.
#[must_use]
pub fn select_cones(
    candidates: &[(Entity, f32)],
    current: &[Entity],
    cap: usize,
    hysteresis: f32,
) -> Vec<Entity> {
    if cap == 0 {
        return Vec::new();
    }
    let mut ranked: Vec<(f32, Entity)> = candidates
        .iter()
        .map(|&(e, s)| {
            (
                if current.contains(&e) {
                    s * hysteresis
                } else {
                    s
                },
                e,
            )
        })
        .collect();
    let better = |a: &(f32, Entity), b: &(f32, Entity)| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1));
    if ranked.len() > cap {
        ranked.select_nth_unstable_by(cap - 1, better);
        ranked.truncate(cap);
    }
    ranked.sort_by(better);
    ranked.into_iter().map(|(_, e)| e).collect()
}

/// The cones the pool made visible, and when it last chose.
#[derive(Resource, Debug, Default)]
pub struct ConePool {
    /// Visible cones (lit and chosen), nearest-by-coverage first at the last selection.
    pub active: Vec<Entity>,
    /// Seconds since the last selection.
    since_select_s: f32,
    /// Quality values at the last selection (lantern switch, headlight switch, cap).
    key: Option<(bool, bool, usize)>,
    /// Seconds since a cone last shone (for the depth prepass).
    idle_s: f32,
    /// Seconds and frames the main camera has had the depth prepass while the fade is on.
    prepass_age_s: f32,
    prepass_frames: u32,
    /// The gate: cones fade in only once the prepass is ready (`0..=1`; 1 with the fade off).
    pub gate: f32,
}

/// What the last selection saw (for `sim.get` and tests).
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq)]
pub struct ConeStats {
    /// Cone entities in the world.
    pub total: usize,
    /// Of those, lit, allowed by the settings and within range in front of the camera.
    pub eligible: usize,
    /// Visible now (within the budget and lit).
    pub active: usize,
    /// Eligible but hidden because of the cap.
    pub over_budget: usize,
    /// Microseconds the last selection took.
    pub select_us: u32,
}

/// The brightness a cone reads from its source (`0..=1`).
fn level_of(
    level: ConeLevel,
    glows: &Query<&SceneryLight>,
    spots: &Query<(&SpotLight, &ScriptLight)>,
) -> f32 {
    match level {
        ConeLevel::Fixed(v) => v,
        ConeLevel::Glow(e) => glows.get(e).map_or(0.0, |l| l.level),
        ConeLevel::Spot(e) => spots.get(e).map_or(0.0, |(spot, script)| {
            if script.base_intensity > 0.0 {
                (spot.intensity / script.base_intensity).clamp(0.0, 1.0)
            } else {
                0.0
            }
        }),
    }
}

/// The prepass gate (`0..=1`): 1 without the depth fade; with it, 0 until the camera has had the
/// prepass for [`PREPASS_READY_S`] and [`PREPASS_READY_FRAMES`], then rising over
/// [`CONE_FADE_IN_S`].
#[must_use]
pub fn prepass_gate(depth_fade: bool, age_s: f32, frames: u32) -> f32 {
    if !depth_fade {
        return 1.0;
    }
    if frames < PREPASS_READY_FRAMES || age_s < PREPASS_READY_S {
        return 0.0;
    }
    ((age_s - PREPASS_READY_S) / CONE_FADE_IN_S).clamp(0.0, 1.0)
}

/// Chooses the visible cones every [`SELECT_INTERVAL_S`] (at once when the quality settings
/// change), and keeps their brightness and visibility current every frame.
#[allow(clippy::too_many_arguments)]
pub fn light_cone_pool_system(
    time: Res<Time<Real>>,
    quality: Res<LightQuality>,
    mut pool: ResMut<ConePool>,
    mut stats: ResMut<ConeStats>,
    cameras: Query<(&Camera, &GlobalTransform, Has<DepthPrepass>), With<crate::MainCamera>>,
    scan: Query<(Entity, &LightCone, &GlobalTransform)>,
    mut apply: Query<(&LightCone, &mut Visibility, &mut MeshTag)>,
    glows: Query<&SceneryLight>,
    spots: Query<(&SpotLight, &ScriptLight)>,
) {
    let cap = quality.cone_cap.min(MAX_CONE_CAP) as usize;
    let key = (quality.lantern_cones, quality.headlight_cones, cap);
    pool.since_select_s += time.delta_secs();
    let camera = cameras
        .iter()
        .find(|(c, _, _)| c.is_active)
        .map(|(_, t, p)| (t.translation(), t.forward().as_vec3(), p));
    // The prepass gate: with the depth fade on, cones wait until the camera has had the prepass
    // for a while, then fade in; with it off they are not gated.
    if quality.cone_depth_fade && camera.is_some_and(|c| c.2) {
        pool.prepass_age_s += time.delta_secs();
        pool.prepass_frames += 1;
    } else {
        pool.prepass_age_s = 0.0;
        pool.prepass_frames = 0;
    }
    let gate = prepass_gate(
        quality.cone_depth_fade,
        pool.prepass_age_s,
        pool.prepass_frames,
    );
    if pool.gate != gate {
        pool.gate = gate;
    }
    if pool.since_select_s >= SELECT_INTERVAL_S || pool.key != Some(key) {
        pool.since_select_s = 0.0;
        pool.key = Some(key);
        let started = std::time::Instant::now();
        let mut candidates: Vec<(Entity, f32)> = Vec::new();
        let mut total = 0;
        if let Some((cam, fwd)) = camera.filter(|_| cap > 0).map(|c| (c.0, c.1)) {
            for (e, cone, global) in &scan {
                total += 1;
                if !cone.kind.allowed(&quality) {
                    continue;
                }
                if level_of(cone.level, &glows, &spots) <= CONE_ON_LEVEL {
                    continue;
                }
                let m = global.affine().matrix3;
                // The axis vector including the lengths: -Z of the matrix.
                let axis = -Vec3::from(m.z_axis);
                let length = axis.length();
                let radius = Vec3::from(m.x_axis).length();
                let apex = global.translation();
                let centre = apex + axis * 0.5;
                let bound = (length * 0.5).max(radius);
                let to_centre = centre - cam;
                if to_centre.length() - bound > cone.kind.max_distance_m()
                    || to_centre.dot(fwd) < -bound
                {
                    continue;
                }
                candidates.push((e, cone_coverage(length, radius, to_centre.length())));
            }
        } else {
            total = scan.iter().count();
        }
        let chosen = select_cones(&candidates, &pool.active, cap, CONE_HYSTERESIS);
        for e in pool.active.iter().filter(|e| !chosen.contains(e)) {
            if let Ok((_, mut vis, _)) = apply.get_mut(*e) {
                if *vis != Visibility::Hidden {
                    *vis = Visibility::Hidden;
                }
            }
        }
        *stats = ConeStats {
            total,
            eligible: candidates.len(),
            active: chosen.len(),
            over_budget: candidates.len().saturating_sub(chosen.len()),
            select_us: started.elapsed().as_micros().min(u128::from(u32::MAX)) as u32,
        };
        pool.active = chosen;
    }
    // Every frame, for the few visible cones: brightness and the on/off state.
    let mut lit = 0;
    pool.active.retain(|&e| {
        let Ok((cone, mut vis, mut tag)) = apply.get_mut(e) else {
            return false;
        };
        let raw = if cone.kind.allowed(&quality) {
            level_of(cone.level, &glows, &spots)
        } else {
            0.0
        };
        if raw > CONE_ON_LEVEL {
            lit += 1;
        }
        let level = raw * gate;
        let on = level > CONE_ON_LEVEL;
        let want = if on {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *vis != want {
            *vis = want;
        }
        if on {
            let packed = pack_tag(
                cone.shape.color,
                level,
                cone.shape.inner_frac,
                cone.shape.ground_h,
            );
            if tag.value != packed {
                tag.value = packed;
            }
        }
        true
    });
    if stats.active != lit {
        stats.active = lit;
    }
}

// ---------------------------------------------------------------------------------------------
// Depth prepass
// ---------------------------------------------------------------------------------------------

/// Marks a [`DepthPrepass`] that [`cone_depth_prepass_system`] added (so only that one is
/// removed again).
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct ConeDepthPrepass;

/// Whether the main camera should have the depth prepass: the depth fade is on, cones are
/// allowed at all, and a cone shines (or shone less than [`PREPASS_RELEASE_S`] ago, while the
/// camera already has it).
#[must_use]
pub fn wants_depth_prepass(
    quality: &LightQuality,
    active_cones: usize,
    idle_s: f32,
    has_prepass: bool,
) -> bool {
    let allowed = quality.cone_depth_fade
        && quality.cone_cap > 0
        && (quality.lantern_cones || quality.headlight_cones);
    allowed && (active_cones > 0 || (has_prepass && idle_s < PREPASS_RELEASE_S))
}

/// Adds the depth prepass to the main camera while the cones need it and removes it again.
pub fn cone_depth_prepass_system(
    mut commands: Commands,
    time: Res<Time<Real>>,
    quality: Res<LightQuality>,
    stats: Res<ConeStats>,
    mut pool: ResMut<ConePool>,
    cameras: Query<(Entity, Has<DepthPrepass>, Has<ConeDepthPrepass>), With<crate::MainCamera>>,
) {
    if stats.active > 0 {
        pool.idle_s = 0.0;
    } else {
        pool.idle_s += time.delta_secs();
    }
    for (camera, has, ours) in &cameras {
        let want = wants_depth_prepass(&quality, stats.active, pool.idle_s, has);
        if want && !has {
            commands
                .entity(camera)
                .insert((DepthPrepass, ConeDepthPrepass));
        } else if !want && has && ours {
            commands
                .entity(camera)
                .remove::<(DepthPrepass, ConeDepthPrepass)>();
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Plugins
// ---------------------------------------------------------------------------------------------

/// The logic of the light cones: budget, brightness, depth prepass. No rendering types are
/// needed, so it works in headless apps.
#[derive(Default)]
pub struct LightConePlugin;

impl Plugin for LightConePlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<LightQualityPlugin>() {
            app.add_plugins(LightQualityPlugin);
        }
        app.init_resource::<ConePool>()
            .init_resource::<ConeStats>()
            .add_systems(
                Update,
                (light_cone_pool_system, cone_depth_prepass_system)
                    .chain()
                    .after(LightQualitySet)
                    .after(SceneryRuntimeSet),
            );
    }
}

mod render;
pub use render::{unit_cone_mesh, ConeAssets, ConeStyle, LightConeMaterial, LightConeRenderPlugin};

#[cfg(test)]
mod tests;
