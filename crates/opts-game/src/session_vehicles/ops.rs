//! The operations on session vehicles: switch the active one, despawn one, put one back at its
//! spawn point, and notice vehicles that vanished. They take `&mut World` so that the pause menu,
//! the quick-switch key and the control protocol run exactly the same code, and so that a test can
//! call them without a window.

use bevy::prelude::*;
use opts_bevy::vehicle_remote::RemoteControls;
use opts_bevy::{ArticulatedChain, CameraMode, CameraState, PlayerVehicle, VehicleController};
use opts_gui::l10n::Localization;
use opts_gui::prelude::ToastBuilder;

use super::registry::{
    DespawnReason, Lifecycle, SessionVehicle, SessionVehicles, SpawnPoint, VehicleId,
};
use crate::in_game::omsi_world::{VehicleComposition, WorldStats};

/// Why an operation was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpError {
    /// No session vehicle has this id.
    NotFound(String),
    /// The vehicle exists but is not in the world (still spawning, failed, gone).
    NotReady(String),
}

impl std::fmt::Display for OpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound(m) | Self::NotReady(m) => f.write_str(m),
        }
    }
}

/// How far the vehicle's place label stays on screen: once the vehicle is further than this from
/// its spawn point the label is gone for good (until a respawn or a new spawn).
pub const PLACE_LABEL_HIDE_DISTANCE_M: f32 = 25.0;

/// Absolute OMSI position of a point of the session frame (the chassis position).
#[must_use]
pub fn local_to_abs(world: &World, local: Vec2) -> (f64, f64) {
    let (tx, ty) = world
        .get_resource::<WorldStats>()
        .map_or((0, 0), |s| s.start_tile);
    let tile = f64::from(opts_bevy::tile_size());
    (
        f64::from(local.x) + f64::from(tx) * tile,
        f64::from(local.y) + f64::from(ty) * tile,
    )
}

/// The session-frame position of an absolute OMSI position.
#[must_use]
pub fn abs_to_local(world: &World, x: f64, y: f64) -> Vec2 {
    let (tx, ty) = world
        .get_resource::<WorldStats>()
        .map_or((0, 0), |s| s.start_tile);
    let tile = f64::from(opts_bevy::tile_size());
    Vec2::new(
        (x - f64::from(tx) * tile) as f32,
        (y - f64::from(ty) * tile) as f32,
    )
}

fn alive_entity(world: &World, id: VehicleId) -> Result<Entity, OpError> {
    let reg = world.resource::<SessionVehicles>();
    let rec = reg
        .get(id)
        .ok_or_else(|| OpError::NotFound(format!("no session vehicle {id}")))?;
    match (rec.lifecycle, rec.entity) {
        (Lifecycle::Alive, Some(e)) if world.get_entity(e).is_ok() => Ok(e),
        (Lifecycle::Spawning, _) => Err(OpError::NotReady(format!(
            "session vehicle {id} is still being spawned"
        ))),
        (l, _) => Err(OpError::NotReady(format!(
            "session vehicle {id} is not in the world ({l:?})"
        ))),
    }
}

/// Parks a vehicle that stops being the active one: pedals released, parking brake on. The
/// physics brings it to rest.
fn park(world: &mut World, entity: Entity) {
    if let Some(mut c) = world.get_mut::<VehicleController>(entity) {
        let inputs = c.inputs_mut();
        inputs.set_throttle(0.0);
        inputs.set_service_brake(0.0);
        inputs.set_steering(0.0);
        inputs.set_parking_brake(true);
    }
}

/// Makes `id` the active vehicle: it takes the driver's input, the cockpit camera, the HUD profile
/// and the script focus; the vehicle that was active is parked (parking brake on) and keeps its
/// state. Switching to the active vehicle does nothing.
///
/// # Errors
/// The vehicle does not exist or is not in the world.
pub fn switch_to(world: &mut World, id: VehicleId) -> Result<(), OpError> {
    let entity = alive_entity(world, id)?;
    let previous = world.resource::<SessionVehicles>().active_id();
    if previous == Some(id) && world.get::<PlayerVehicle>(entity).is_some() {
        return Ok(());
    }
    // Whatever carries the marker loses it (also a stray one), then the new vehicle takes it.
    let marked: Vec<Entity> = world
        .query_filtered::<Entity, With<PlayerVehicle>>()
        .iter(world)
        .filter(|e| *e != entity)
        .collect();
    for e in marked {
        park(world, e);
        world.entity_mut(e).remove::<PlayerVehicle>();
    }
    world.entity_mut(entity).insert(PlayerVehicle);
    world
        .resource_mut::<SessionVehicles>()
        .set_active(Some(id));
    if let Some(mut remote) = world.get_resource_mut::<RemoteControls>() {
        // Held pedals of the old vehicle must not drive the new one.
        remote.release_all();
    }
    let (name, std_camera) = {
        let reg = world.resource::<SessionVehicles>();
        (
            reg.get(id).map(|r| r.name.clone()).unwrap_or_default(),
            world
                .get::<opts_bevy::BusVehicle>(entity)
                .map_or(0, |b| b.std_camera_index),
        )
    };
    if let Some(mut cam) = world.get_resource_mut::<CameraState>() {
        cam.sub_camera_index = std_camera;
        cam.view_yaw = 0.0;
        cam.view_pitch = 0.0;
    }
    if let Some(mut stats) = world.get_resource_mut::<WorldStats>() {
        stats.vehicle_name = name.clone();
    }
    tracing::info!(id, vehicle = %name, "active session vehicle switched");
    Ok(())
}

/// Switches `step` places through the living vehicles (the quick-switch key).
///
/// # Errors
/// There is no other vehicle to switch to.
pub fn switch_step(world: &mut World, step: i32) -> Result<VehicleId, OpError> {
    let target = world
        .resource::<SessionVehicles>()
        .cycle(step)
        .ok_or_else(|| OpError::NotReady("there is no session vehicle".into()))?;
    switch_to(world, target)?;
    Ok(target)
}

/// All entities that belong to a vehicle: its root, the chain segments and the composition parts.
fn vehicle_parts(world: &World, root: Entity) -> Vec<Entity> {
    let mut parts = vec![root];
    if let Some(chain) = world.get::<ArticulatedChain>(root) {
        parts.extend(chain.segments.iter().copied());
    }
    if let Some(comp) = world.get::<VehicleComposition>(root) {
        parts.extend(comp.parts().map(|p| p.entity));
    }
    parts.sort();
    parts.dedup();
    parts
}

/// What a despawn changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DespawnOutcome {
    /// The vehicle that is active now (`None`: no vehicle, free camera).
    pub active: Option<VehicleId>,
    /// Entities removed (root, trailers, composition parts).
    pub removed: usize,
}

/// Removes a session vehicle with its trailer chain and every entity below it. Despawning the
/// active vehicle makes the most recently spawned other one active, or none (free camera).
/// A vehicle still waiting to be spawned is cancelled.
///
/// # Errors
/// The vehicle does not exist or is already gone.
pub fn despawn(
    world: &mut World,
    id: VehicleId,
    reason: DespawnReason,
) -> Result<DespawnOutcome, OpError> {
    let (lifecycle, entity, was_active) = {
        let reg = world.resource::<SessionVehicles>();
        let rec = reg
            .get(id)
            .ok_or_else(|| OpError::NotFound(format!("no session vehicle {id}")))?;
        (rec.lifecycle, rec.entity, reg.active_id() == Some(id))
    };
    let mut removed = 0;
    match lifecycle {
        Lifecycle::Despawned => {
            return Err(OpError::NotReady(format!(
                "session vehicle {id} is already despawned"
            )))
        }
        Lifecycle::Spawning | Lifecycle::Failed => {
            world
                .resource_mut::<SessionVehicles>()
                .mark_despawned(id, DespawnReason::Cancelled);
            return Ok(DespawnOutcome {
                active: world.resource::<SessionVehicles>().active_id(),
                removed,
            });
        }
        Lifecycle::Alive => {}
    }
    let successor = was_active
        .then(|| world.resource::<SessionVehicles>().successor_of(id))
        .flatten();
    if let Some(root) = entity.filter(|e| world.get_entity(*e).is_ok()) {
        for part in vehicle_parts(world, root) {
            if world.get_entity(part).is_ok() {
                world.entity_mut(part).despawn();
                removed += 1;
            }
        }
    }
    world
        .resource_mut::<SessionVehicles>()
        .mark_despawned(id, reason);
    tracing::info!(id, ?reason, removed, "session vehicle despawned");
    if was_active {
        match successor {
            Some(next) => {
                switch_to(world, next)?;
            }
            None => leave_vehicle_mode(world),
        }
    }
    Ok(DespawnOutcome {
        active: world.resource::<SessionVehicles>().active_id(),
        removed,
    })
}

/// No vehicle is active any more: the camera is free where it is, the HUD loses its instruments.
fn leave_vehicle_mode(world: &mut World) {
    let transform = world
        .query_filtered::<&Transform, With<Camera3d>>()
        .iter(world)
        .next()
        .copied();
    if let Some(mut stats) = world.get_resource_mut::<WorldStats>() {
        stats.vehicle_name.clear();
    }
    if let Some(mut remote) = world.get_resource_mut::<RemoteControls>() {
        remote.release_all();
    }
    let Some(mut cam) = world.get_resource_mut::<CameraState>() else {
        return;
    };
    if cam.mode != CameraMode::FreeRoam {
        cam.previous_mode = cam.mode;
        cam.mode = CameraMode::FreeRoam;
    }
    if let Some(t) = transform {
        let (yaw, pitch, _) = t.rotation.to_euler(EulerRot::YXZ);
        cam.free_roam_spawn = t.translation;
        cam.free_roam_spawn_yaw = yaw;
        cam.free_roam_spawn_pitch = pitch;
        cam.free_roam_yaw = yaw;
        cam.free_roam_pitch = pitch;
    }
}

/// Puts a vehicle back at its spawn point, at rest, and shows its place label again. The trailer
/// chain re-seeds behind it (the articulation system treats the jump as a placement).
///
/// # Errors
/// The vehicle does not exist, is not in the world or has no recorded spawn point.
pub fn respawn(world: &mut World, id: VehicleId) -> Result<(), OpError> {
    let entity = alive_entity(world, id)?;
    let point: SpawnPoint = world
        .resource::<SessionVehicles>()
        .get(id)
        .and_then(|r| r.spawn_point)
        .ok_or_else(|| OpError::NotReady(format!("session vehicle {id} has no spawn point")))?;
    let local = abs_to_local(world, point.x, point.y);
    let active = world.resource::<SessionVehicles>().active_id() == Some(id);
    let Some(mut c) = world.get_mut::<VehicleController>(entity) else {
        return Err(OpError::NotReady(format!(
            "session vehicle {id} has no simulation"
        )));
    };
    let ch = &mut c.sim.chassis;
    ch.position = local;
    ch.heading_rad = (point.heading_deg as f32).to_radians();
    ch.elevation_z = point.z as f32;
    ch.local_velocity = Vec2::ZERO;
    ch.yaw_rate_rad_s = 0.0;
    ch.local_acceleration = Vec2::ZERO;
    c.render_prev = None;
    let inputs = c.inputs_mut();
    inputs.set_throttle(0.0);
    inputs.set_service_brake(0.0);
    inputs.set_steering(0.0);
    if let Some(r) = world.resource_mut::<SessionVehicles>().get_mut(id) {
        r.place_dismissed = false;
    }
    if active {
        if let Some(mut remote) = world.get_resource_mut::<RemoteControls>() {
            remote.release_all();
            remote.restart_odometer();
        }
    }
    tracing::info!(id, "session vehicle respawned at its spawn point");
    Ok(())
}

/// Notices vehicles whose entity vanished without a request (destroyed, or removed with a tile):
/// they are marked [`DespawnReason::Lost`], the next vehicle becomes active and a warning toast
/// names the vehicle. No entity of a lost vehicle is left dangling (its trailer parts go too).
pub fn sync_lost(world: &mut World) {
    let lost: Vec<VehicleId> = {
        let reg = world.resource::<SessionVehicles>();
        reg.records()
            .iter()
            .filter(|r| r.lifecycle == Lifecycle::Alive)
            .filter(|r| r.entity.is_none_or(|e| world.get_entity(e).is_err()))
            .map(|r| r.id)
            .collect()
    };
    for id in lost {
        let (name, trailers) = {
            let reg = world.resource::<SessionVehicles>();
            reg.get(id)
                .map(|r| (r.name.clone(), r.trailers.clone()))
                .unwrap_or_default()
        };
        for t in trailers {
            if world.get_entity(t).is_ok() {
                world.entity_mut(t).despawn();
            }
        }
        let _ = despawn(world, id, DespawnReason::Lost);
        tracing::warn!(id, vehicle = %name, "a session vehicle vanished from the world");
        let text = world.get_resource::<Localization>().map_or_else(
            || name.clone(),
            |l| {
                l.t_args(
                    crate::shell::keys::VEHICLE_LOST,
                    &[("name".to_owned(), name.as_str().into())],
                )
            },
        );
        let mut commands = world.commands();
        ToastBuilder::new(text)
            .with_duration(6.0)
            .spawn(&mut commands);
        world.flush();
    }
}

/// Refreshes the trailer lists of the records from the live chains, and the latch of the place
/// label: a vehicle that has been further than [`PLACE_LABEL_HIDE_DISTANCE_M`] from its spawn
/// point no longer shows it.
pub fn refresh_records(world: &mut World) {
    let ids: Vec<(VehicleId, Entity, Option<SpawnPoint>)> = world
        .resource::<SessionVehicles>()
        .records()
        .iter()
        .filter(|r| r.lifecycle == Lifecycle::Alive)
        .filter_map(|r| Some((r.id, r.entity?, r.spawn_point)))
        .collect();
    for (id, entity, point) in ids {
        let trailers: Vec<Entity> = world
            .get::<ArticulatedChain>(entity)
            .map(|c| c.segments.clone())
            .unwrap_or_default();
        let away = point.and_then(|p| {
            let c = world.get::<VehicleController>(entity)?;
            let (x, y) = local_to_abs(world, c.sim.chassis.position);
            let (dx, dy) = (x - p.x, y - p.y);
            Some(((dx * dx + dy * dy).sqrt() as f32) > PLACE_LABEL_HIDE_DISTANCE_M)
        });
        if let Some(rec) = world.resource_mut::<SessionVehicles>().get_mut(id) {
            if rec.trailers != trailers {
                rec.trailers = trailers;
            }
            if away == Some(true) && !rec.place_dismissed {
                rec.place_dismissed = true;
            }
        }
    }
}

/// Entity of the session vehicle with this id, if it is in the world.
#[must_use]
pub fn entity_of(world: &World, id: VehicleId) -> Option<Entity> {
    alive_entity(world, id).ok()
}

/// The id a vehicle entity carries.
#[must_use]
pub fn id_of(world: &World, entity: Entity) -> Option<VehicleId> {
    world.get::<SessionVehicle>(entity).map(|s| s.0)
}
