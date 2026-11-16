//! The engine exhaust plume of the player's vehicle (M-0117): puffs from the back of the vehicle,
//! emitted at the rate of [`opts_sim::exhaust::puff_rate_per_s`] (air temperature and throttle),
//! at most `graphics.exhaust_particles` alive at once, and off with `graphics.exhaust = off`.
//!
//! The anchor is a fallback until the outlet is read from the vehicle data: 1 m above the ground
//! and [`REAR_OFFSET_M`] behind the vehicle's origin. Each puff rises and widens on its own
//! straight path and is removed after [`opts_sim::exhaust::PUFF_LIFETIME_S`].

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use opts_bevy::{PlayerVehicle, SessionEnvironment, VehicleController};
use opts_sim::exhaust::{puff_rate_per_s, puffs_to_spawn, PUFF_LIFETIME_S};

use crate::settings::GameSettings;
use crate::settings_registry::lighting::{exhaust_cap_of, exhaust_on_of};

/// Fallback distance (m) of the exhaust outlet behind the vehicle's origin.
pub const REAR_OFFSET_M: f32 = 5.5;
/// Fallback height (m) of the exhaust outlet above the vehicle's origin.
pub const OUTLET_HEIGHT_M: f32 = 1.0;
/// Edge length (m) of a puff at birth; it grows to twice this over its life.
const PUFF_SIZE_M: f32 = 0.35;
/// Velocity (m/s) of a puff: up and back, slightly to the side of the fall of the air.
const DRIFT_M_PER_S: Vec3 = Vec3::new(0.0, 0.8, -0.6);

/// One visible puff: its age and the point it left the outlet from.
#[derive(Component, Debug, Clone, Copy)]
pub struct Puff {
    age: f32,
    origin: Vec3,
    drift: Vec3,
}

/// The emitter's carried fraction of a puff and the shared mesh and material of the puffs.
#[derive(Resource, Default)]
pub struct ExhaustState {
    carry: f32,
    assets: Option<(Handle<Mesh>, Handle<StandardMaterial>)>,
}

/// The fallback outlet of a vehicle at `vehicle`: behind it and above the ground.
#[must_use]
pub fn outlet_of(vehicle: &GlobalTransform) -> Vec3 {
    let t = vehicle.compute_transform();
    t.translation - t.forward() * REAR_OFFSET_M + Vec3::Y * OUTLET_HEIGHT_M
}

/// Where a puff of age `age` is: it rises and drifts back with the wind direction `drift`.
#[must_use]
pub fn puff_position(puff: &Puff) -> Vec3 {
    puff.origin + puff.drift * puff.age
}

/// The read-only inputs of the exhaust: the frame time, the settings and the session weather.
#[derive(SystemParam)]
pub struct ExhaustInputs<'w> {
    time: Res<'w, Time>,
    settings: Res<'w, GameSettings>,
    env: Res<'w, SessionEnvironment>,
}

/// The mesh and material stores the puffs are built from.
#[derive(SystemParam)]
pub struct ExhaustAssets<'w> {
    meshes: ResMut<'w, Assets<Mesh>>,
    materials: ResMut<'w, Assets<StandardMaterial>>,
}

/// Ages, moves and removes the puffs, then emits the new ones of the player's vehicle.
pub fn update_exhaust(
    inputs: ExhaustInputs,
    mut state: ResMut<ExhaustState>,
    mut assets: ExhaustAssets,
    vehicles: Query<(&GlobalTransform, &VehicleController), With<PlayerVehicle>>,
    mut puffs: Query<(Entity, &mut Puff, &mut Transform)>,
    mut commands: Commands,
) {
    let dt = inputs.time.delta_secs();
    let live = age_puffs(dt, &mut puffs, &mut commands);
    if !exhaust_on_of(&inputs.settings) {
        return;
    }
    let Ok((outlet, controller)) = vehicles.single() else {
        return;
    };
    let rate = puff_rate_per_s(
        inputs.env.conditions().temperature_c,
        controller.inputs().throttle(),
    );
    let (count, carry) = puffs_to_spawn(
        rate,
        dt,
        live,
        exhaust_cap_of(&inputs.settings),
        state.carry,
    );
    state.carry = carry;
    if count == 0 {
        return;
    }
    let (mesh, material) = state
        .assets
        .get_or_insert_with(|| puff_assets(&mut assets.meshes, &mut assets.materials))
        .clone();
    let origin = outlet_of(outlet);
    for _ in 0..count {
        commands.spawn((
            Puff {
                age: 0.0,
                origin,
                drift: DRIFT_M_PER_S,
            },
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material.clone()),
            Transform::from_translation(origin).with_scale(Vec3::splat(PUFF_SIZE_M)),
        ));
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
