//! The per-tick update: removes the puddles the target or the distance drops, then spawns new ones
//! through [`super::spawn`].

use bevy::prelude::*;
use opts_bevy::reflections::Wetness;

use crate::in_game::omsi_world::GroundIndex;
use crate::in_game::types::InGameCamera;

use super::super::surfaces::wanted_state;
use super::spawn::{ensure_assets, spawn_new};
use super::{puddle_target, Puddle, PuddleState, DROP_DISTANCE_M, SKY_NITS, TICK_S};

/// What [`update_puddles`] reads besides the weather: wetness, the ground and the camera.
#[derive(bevy::ecs::system::SystemParam)]
pub struct PuddleInputs<'w, 's> {
    wetness: Res<'w, Wetness>,
    ground: Option<Res<'w, GroundIndex>>,
    cameras: Query<'w, 's, &'static GlobalTransform, With<InGameCamera>>,
}

/// Adds, keeps and removes the puddles.
pub fn update_puddles(
    scene: super::super::WeatherScene,
    assets: super::super::WeatherAssets,
    inputs: PuddleInputs,
    mut state: ResMut<PuddleState>,
    puddles: Query<(Entity, &Puddle)>,
    mut commands: Commands,
) {
    let super::super::WeatherScene { time, env, effects } = scene;
    let super::super::WeatherAssets {
        mut images,
        mut meshes,
        mut materials,
    } = assets;
    let PuddleInputs {
        wetness,
        ground,
        cameras,
    } = inputs;
    state.timer += time.delta_secs();
    if state.timer < TICK_S {
        return;
    }
    state.timer = 0.0;
    let surface = wanted_state(&env, &effects);
    let target = puddle_target(&surface, wetness.0);
    let camera = cameras.iter().next().map(|c| c.translation());
    let centre = camera.map(|c| Vec2::new(c.x, -c.z));
    // Remove the far ones and those above the target (the farthest first).
    let mut present: Vec<(Entity, f32)> = puddles
        .iter()
        .map(|(e, p)| (e, centre.map_or(0.0, |c| c.distance(p.at))))
        .collect();
    present.sort_by(|a, b| b.1.total_cmp(&a.1));
    let mut count = present.len();
    for (e, d) in &present {
        if count > target || *d > DROP_DISTANCE_M {
            commands.entity(*e).despawn();
            count -= 1;
        }
    }
    let (Some(ground), Some(camera), Some(centre)) = (ground, camera, centre) else {
        return;
    };
    if count >= target {
        return;
    }
    ensure_assets(&mut state, (&mut meshes, &mut materials, &mut images));
    let (Some(mesh), Some(material)) = (state.mesh.clone(), state.material.clone()) else {
        return;
    };
    // The water mirrors the sky: pale grey under cloud, bluer when it is clear, as bright as the
    // light (the cloud brightness follows the sun and the cover), not black blobs.
    let look = opts_bevy::weather::weather_look(env.conditions(), [0.5; 3], env.sun_alt_deg());
    let clear = (1.0 - look.cloud).clamp(0.0, 1.0);
    let sky = LinearRgba::rgb(0.8 - 0.25 * clear, 0.85 - 0.12 * clear, 0.95)
        * (SKY_NITS * look.cloud.max(0.15));
    if let Some(mut m) = materials.get_mut(&material) {
        if m.emissive != sky {
            m.emissive = sky;
        }
    }
    spawn_new(
        &mut commands,
        &mut state,
        &ground,
        (camera, centre),
        (mesh, material, count, target),
    );
}
