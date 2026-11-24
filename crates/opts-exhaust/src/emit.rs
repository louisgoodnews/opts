//! The exhaust system: ages and moves the puffs, picks the emitters nearest the camera and spawns
//! their puffs each frame. Driven by [`update_exhaust`]; the choice and the fade are in `nearest`.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use opts_bevy::vehicle::ExhaustOutlet;
use opts_bevy::{ArticulatedChain, MainCamera, SessionEnvironment};
use opts_sim::exhaust::{puffs_to_spawn, PUFF_LIFETIME_S};

use crate::select::{emitters, Vehicles};
use crate::{puff_position, ExhaustSettings, ExhaustState, Puff, DRIFT_M_PER_S, PUFF_SIZE_M};

/// The read-only inputs of the exhaust: the frame time, the settings, the session weather, the
/// camera and the articulated chains of the vehicles.
#[derive(SystemParam)]
pub struct ExhaustInputs<'w, 's> {
    time: Res<'w, Time>,
    settings: Res<'w, ExhaustSettings>,
    env: Res<'w, SessionEnvironment>,
    cameras: Query<'w, 's, &'static GlobalTransform, With<MainCamera>>,
    chains: Query<'w, 's, &'static ArticulatedChain>,
}

/// The mesh and material stores the puffs are built from.
#[derive(SystemParam)]
pub struct ExhaustAssets<'w> {
    meshes: ResMut<'w, Assets<Mesh>>,
    materials: ResMut<'w, Assets<StandardMaterial>>,
}

/// Ages, moves and removes the puffs, then emits the new ones of the emitters nearest the camera.
pub fn update_exhaust(
    inputs: ExhaustInputs,
    mut state: ResMut<ExhaustState>,
    mut assets: ExhaustAssets,
    vehicles: Vehicles,
    units: Query<(&GlobalTransform, Option<&ExhaustOutlet>)>,
    mut puffs: Query<(Entity, &mut Puff, &mut Transform)>,
    mut commands: Commands,
) {
    let dt = inputs.time.delta_secs();
    let mut live = age_puffs(dt, &mut puffs, &mut commands);
    if !inputs.settings.on {
        return;
    }
    let Ok(camera) = inputs.cameras.single() else {
        return;
    };
    let temp_c = inputs.env.conditions().temperature_c;
    let emitters = emitters(
        camera.translation(),
        temp_c,
        &vehicles,
        &inputs.chains,
        &units,
    );
    state
        .carry
        .retain(|e, _| emitters.iter().any(|m| m.entity == *e));
    let (mesh, material) = state
        .assets
        .get_or_insert_with(|| puff_assets(&mut assets.meshes, &mut assets.materials))
        .clone();
    for emitter in emitters {
        let carry = state.carry.get(&emitter.entity).copied().unwrap_or(0.0);
        let (count, carry) = puffs_to_spawn(emitter.rate, dt, live, inputs.settings.cap, carry);
        state.carry.insert(emitter.entity, carry);
        live += count;
        for _ in 0..count {
            commands.spawn((
                Puff {
                    age: 0.0,
                    origin: emitter.outlet,
                    drift: DRIFT_M_PER_S,
                },
                Mesh3d(mesh.clone()),
                MeshMaterial3d(material.clone()),
                Transform::from_translation(emitter.outlet).with_scale(Vec3::splat(PUFF_SIZE_M)),
            ));
        }
    }
}

/// Ages every puff by `dt`, moves it, grows it and removes the expired ones. Returns how many
/// puffs stay alive.
fn age_puffs(
    dt: f32,
    puffs: &mut Query<(Entity, &mut Puff, &mut Transform)>,
    commands: &mut Commands,
) -> u32 {
    let mut live = 0;
    for (entity, mut puff, mut transform) in puffs.iter_mut() {
        puff.age += dt;
        if puff.age >= PUFF_LIFETIME_S {
            commands.entity(entity).despawn();
            continue;
        }
        live += 1;
        transform.translation = puff_position(&puff);
        let grow = 1.0 + puff.age / PUFF_LIFETIME_S;
        transform.scale = Vec3::splat(PUFF_SIZE_M * grow);
    }
    live
}

/// The shared mesh (a sphere) and the translucent grey material of the puffs.
fn puff_assets(
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) -> (Handle<Mesh>, Handle<StandardMaterial>) {
    let mesh = meshes.add(Sphere::new(0.5));
    let material = materials.add(StandardMaterial {
        base_color: Color::srgba(0.82, 0.82, 0.8, 0.45),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });
    (mesh, material)
}
