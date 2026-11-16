//! Spawning puddles: the shared plane and water material, and the tick's placement on level road.

use std::f32::consts::TAU;

use bevy::light::{NotShadowCaster, NotShadowReceiver};
use bevy::prelude::*;

use crate::in_game::omsi_world::GroundIndex;
use crate::in_game::types::InGameEntity;

use super::super::precip;
use super::{
    blob_alpha, Puddle, PuddleState, FLAT_TOLERANCE_M, LIFT_M, MAX_DISTANCE_M, MIN_DISTANCE_M,
    SPAWN_PER_TICK, TRIES_PER_TICK,
};

/// The shared plane and the dark water material, made on first use.
pub(super) fn ensure_assets(
    state: &mut PuddleState,
    (meshes, materials, images): (
        &mut Assets<Mesh>,
        &mut Assets<StandardMaterial>,
        &mut Assets<Image>,
    ),
) {
    if state.mesh.is_none() {
        state.mesh = Some(meshes.add(Plane3d::default().mesh().size(1.0, 1.0)));
        let blob = images.add(precip::texture(64, 64, blob_alpha));
        state.material = Some(materials.add(StandardMaterial {
            base_color: Color::linear_rgba(0.012, 0.016, 0.02, 0.92),
            base_color_texture: Some(blob),
            alpha_mode: AlphaMode::Blend,
            perceptual_roughness: 0.03,
            reflectance: 0.9,
            cull_mode: None,
            double_sided: true,
            // Roads draw with a depth bias of 4: above them.
            depth_bias: 8.0,
            ..default()
        }));
    }
}

/// Tries to place new puddles around the camera on level road, up to the per-tick limit and the
/// target.
pub(super) fn spawn_new(
    commands: &mut Commands,
    state: &mut PuddleState,
    ground: &GroundIndex,
    (camera, centre): (Vec3, Vec2),
    (mesh, material, count, target): (Handle<Mesh>, Handle<StandardMaterial>, usize, usize),
) {
    let mut added = 0;
    for _ in 0..TRIES_PER_TICK {
        if added >= SPAWN_PER_TICK || count + added >= target {
            break;
        }
        let angle = state.next() * TAU;
        let distance = MIN_DISTANCE_M + state.next() * (MAX_DISTANCE_M - MIN_DISTANCE_M);
        let at = centre + Vec2::new(angle.cos(), angle.sin()) * distance;
        let hint = camera.y - 1.4;
        let Some(z) = ground.road_height_at(at.x, at.y, hint) else {
            continue;
        };
        let (sx, sz) = (0.9 + state.next() * 2.6, 0.7 + state.next() * 1.8);
        let yaw = state.next() * TAU;
        // Level across the patch: the road at the ends of both axes is as high as at the middle.
        let (sin, cos) = yaw.sin_cos();
        let level = [(sx, 0.0), (-sx, 0.0), (0.0, sz), (0.0, -sz)]
            .iter()
            .all(|(dx, dy)| {
                let p = at + Vec2::new(cos * dx - sin * dy, sin * dx + cos * dy) * 0.5;
                ground
                    .road_height_at(p.x, p.y, z)
                    .is_some_and(|h| (h - z).abs() <= FLAT_TOLERANCE_M)
            });
        if !level {
            continue;
        }
        commands.spawn((
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material.clone()),
            Transform {
                translation: Vec3::new(at.x, z + LIFT_M, -at.y),
                rotation: Quat::from_rotation_y(yaw),
                scale: Vec3::new(sx, 1.0, sz),
            },
            NotShadowCaster,
            NotShadowReceiver,
            Puddle { at },
            InGameEntity,
            Name::new("Puddle"),
        ));
        added += 1;
    }
}
