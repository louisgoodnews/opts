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

mod pool;
mod shape;
pub use pool::*;
pub use shape::{
    cone_angles, headlight_cone_shape, lantern_cone_length, lantern_cone_shape,
    spotlight_cone_shape, ConeKind, ConeShape,
};

/// The ground clip height for an apex `y` metres above the object's origin (which stands on the
/// ground); none when the apex is not clearly above it.
pub(super) fn ground_height(y: f32) -> Option<f32> {
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
