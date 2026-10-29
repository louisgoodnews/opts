//! Light effects of vehicles (`[light_enh]`, `[light_enh_2]` in `model.cfg`) and a small pool of
//! real headlights.
//!
//! * **Glow sprites:** every light effect becomes a sprite under the vehicle's root (not under a
//!   LOD group, so it never pops with the meshes), drawn by the scenery light system
//!   ([`crate::scenery_runtime::scenery_light_system`]: size by brightness, fade by the cone).
//!   Brightness = variable × factor, clamped to `0..=1`, approached with the block's time
//!   constant. The variable comes from the vehicle's scripts or its AI variable table
//!   ([`VariableSource`]), so player and AI vehicles share the code.
//! * **Headlight pool:** forward-facing white light effects mark a vehicle's headlights. Of the
//!   vehicles whose headlights shine, the nearest [`HEADLIGHT_POOL`] get one real, shadowless
//!   `SpotLight` each (at the middle of the pair), with hysteresis so the choice is stable under
//!   small camera moves. Everything else is only a glow (hundreds of spot lights would cost too
//!   much).
//!
//! Design: `docs/src/architecture/ai-traffic-kinematics.md`, §5.

use bevy::light::{NotShadowCaster, NotShadowReceiver};
use bevy::prelude::*;
use opts_core::formats::scenery::ScoLight;
use opts_core::formats::vehicles::model::ModelConfig;

use crate::ai_variables::{AiVariables, VariableSource};
use crate::coords::omsi_to_bevy_pos;
use crate::scenery_runtime::{SceneryLight, SceneryLightAssets, VarRef};
use crate::vehicle_script::VehicleScripts;

/// Number of real headlights (one spot light per vehicle).
pub const HEADLIGHT_POOL: usize = 6;
/// Headlights shine (for the pool) above this brightness.
const HEADLIGHT_ON: f32 = 0.3;

/// A light effect of a vehicle; the entity also carries the [`SceneryLight`] that is drawn.
#[derive(Component, Debug, Clone)]
pub struct VehicleGlow {
    /// The vehicle root whose scripts or AI variables drive the light.
    pub owner: Entity,
    /// Brightness variable (empty: always on).
    pub variable: String,
    /// Multiplier on the variable.
    pub factor: f32,
    /// Seconds to reach 63 % of a new brightness (`0` = at once).
    pub time_const_s: f32,
    /// Colour, 0–255.
    pub color: [f32; 3],
    /// The light is a headlight (shines forward, white).
    pub headlight: bool,
}

impl VehicleGlow {
    /// The brightness after `dt` seconds towards `target` from `level`.
    #[must_use]
    pub fn approach(&self, level: f32, target: f32, dt: f32) -> f32 {
        if self.time_const_s <= 1e-3 {
            target
        } else {
            level + (target - level) * (1.0 - (-dt / self.time_const_s).exp())
        }
    }
}

/// Whether a light effect is a headlight: directed forward (OMSI +y) and white-ish.
#[must_use]
pub fn is_headlight(light: &ScoLight) -> bool {
    !light.omni && light.direction.y > 0.7 && light.color.iter().all(|c| *c >= 150.0)
}

/// The light effects of `model` that belong to the meshes drawn up close: those outside any
/// `[LOD]` group and those of the detail group(s) ([`ModelConfig::detail_lod`]).
pub fn detail_lights(model: &ModelConfig) -> impl Iterator<Item = &ScoLight> {
    let first = model.lods.first().map(|l| l.threshold);
    let second = match (model.lods.first(), model.lods.get(1)) {
        (Some(a), Some(b)) if a.threshold <= 0.0 && b.threshold > 0.0 => Some(b.threshold),
        _ => None,
    };
    model.lights.iter().filter(move |l| {
        l.lod_threshold.is_none() || l.lod_threshold == first || l.lod_threshold == second
    })
}

/// Spawns the glow sprites of `model`'s light effects as children of `parent` (the vehicle unit
/// they sit on), driven by the variables of `owner` (the vehicle root). Mesh and material are
/// added by [`attach_glow_assets`]. Returns the sprite entities.
pub fn spawn_vehicle_glows(
    commands: &mut Commands,
    owner: Entity,
    parent: Entity,
    model: &ModelConfig,
) -> Vec<Entity> {
    spawn_vehicle_glows_with_cones(commands, owner, parent, model, true)
}

/// [`spawn_vehicle_glows`], with the choice whether the headlights also get a visible light
/// cone. The player vehicle passes `false` when its model has `[spotlight]`s: their cones (see
/// `spawn_vehicle_model`) are the more exact data, and the two must not show twice.
pub fn spawn_vehicle_glows_with_cones(
    commands: &mut Commands,
    owner: Entity,
    parent: Entity,
    model: &ModelConfig,
    headlight_cones: bool,
) -> Vec<Entity> {
    let half = |deg: f32| (deg.clamp(0.0, 360.0) * 0.5).to_radians().cos();
    let mut out = Vec::new();
    for light in detail_lights(model) {
        let sprite = SceneryLight {
            var: VarRef::One,
            factor: light.factor,
            size_m: light.size_m.max(0.01),
            direction: if light.omni {
                Vec3::ZERO
            } else {
                omsi_to_bevy_pos(light.direction).normalize_or_zero()
            },
            cos_inner: half(light.cone_inner_deg),
            cos_outer: half(light.cone_outer_deg),
            level: 0.0,
        };
        let glow = VehicleGlow {
            owner,
            variable: light.variable.trim().to_owned(),
            factor: light.factor,
            time_const_s: light.time_const_s.max(0.0),
            color: light.color,
            headlight: is_headlight(light),
        };
        let e = commands
            .spawn((
                Transform::from_translation(omsi_to_bevy_pos(light.position))
                    .with_scale(Vec3::splat(sprite.size_m)),
                Visibility::Hidden,
                NotShadowCaster,
                NotShadowReceiver,
                sprite,
                glow,
                Name::new("VehicleGlow"),
            ))
            .id();
        commands.entity(parent).add_child(e);
        out.push(e);
        if headlight_cones && is_headlight(light) {
            if let Some(shape) = crate::light_cones::headlight_cone_shape(light) {
                crate::light_cones::spawn_light_cone(
                    commands,
                    parent,
                    crate::light_cones::LightCone {
                        kind: crate::light_cones::ConeKind::Headlight,
                        shape,
                        level: crate::light_cones::ConeLevel::Glow(e),
                    },
                );
            }
        }
    }
    out
}

/// Shared sprite mesh and per-colour materials of vehicle glows.
#[derive(Resource, Default)]
pub struct VehicleGlowAssets(pub SceneryLightAssets);

/// Gives new glow sprites their mesh and material.
pub fn attach_glow_assets(
    mut commands: Commands,
    fresh: Query<(Entity, &VehicleGlow), Without<Mesh3d>>,
    mut assets: ResMut<VehicleGlowAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (e, glow) in &fresh {
        let mesh = assets.0.glow_mesh(&mut meshes);
        let material = assets.0.material(&mut materials, glow.color);
        commands
            .entity(e)
            .insert((Mesh3d(mesh), MeshMaterial3d(material)));
    }
}

/// Sets each glow's brightness from its owner's variable, smoothed by its time constant.
pub fn vehicle_glow_system(
    time: Res<Time<Real>>,
    vehicles: Query<(Option<&VehicleScripts>, Option<&AiVariables>)>,
    mut glows: Query<(&VehicleGlow, &mut SceneryLight)>,
) {
    let dt = time.delta_secs().min(0.5);
    for (glow, mut light) in &mut glows {
        let Ok((scripts, table)) = vehicles.get(glow.owner) else {
            continue;
        };
        let value = if glow.variable.is_empty() {
            1.0
        } else {
            let Some(source) = VariableSource::of(scripts, table) else {
                continue;
            };
            source.get(&glow.variable)
        };
        let target = (value * glow.factor).clamp(0.0, 1.0);
        let level = glow.approach(light.level, target, dt);
        let level = if (level - target).abs() < 1e-3 {
            target
        } else {
            level
        };
        if light.level != level {
            light.level = level;
        }
    }
}

mod headlight_pool;
pub use headlight_pool::{
    choose_headlights, headlight_pool_system, PooledHeadlight, HEADLIGHT_HYSTERESIS,
};

/// Registers the vehicle glow and headlight pool systems.
#[derive(Default)]
pub struct VehicleGlowPlugin;

impl Plugin for VehicleGlowPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<VehicleGlowAssets>().add_systems(
            Update,
            (
                attach_glow_assets,
                vehicle_glow_system.before(crate::scenery_runtime::SceneryRuntimeSet),
                headlight_pool_system,
            ),
        );
    }
}

#[cfg(test)]
mod tests;
