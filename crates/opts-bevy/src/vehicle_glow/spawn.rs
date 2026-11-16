//! Spawning the glow sprites of a vehicle's light effects, and the cones of its headlights.

use bevy::light::{NotShadowCaster, NotShadowReceiver};
use bevy::prelude::*;

use crate::coords::omsi_to_bevy_pos;
use crate::scenery_runtime::{SceneryLight, VarRef};

use super::{is_headlight, ModelConfig, ScoLight, VehicleGlow};

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
/// added by [`super::attach_glow_assets`]. Returns the sprite entities.
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
