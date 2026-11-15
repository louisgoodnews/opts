//! Putting a session vehicle back at a spawn point, at rest.

use bevy::prelude::*;
use opts_bevy::vehicle_remote::RemoteControls;
use opts_bevy::VehicleController;

use crate::session_vehicles::registry::{SessionVehicles, SpawnPoint, VehicleId};

use super::frame::{abs_to_local, alive_entity, OpError};

/// Puts a vehicle back at its spawn point, at rest, and shows its place label again. The trailer
/// chain re-seeds behind it (the articulation system treats the jump as a placement).
///
/// # Errors
/// The vehicle does not exist, is not in the world or has no recorded spawn point.
pub fn respawn(world: &mut World, id: VehicleId) -> Result<(), OpError> {
    let point: SpawnPoint = world
        .resource::<SessionVehicles>()
        .get(id)
        .and_then(|r| r.spawn_point)
        .ok_or_else(|| OpError::NotReady(format!("session vehicle {id} has no spawn point")))?;
    respawn_at(world, id, point)
}

/// Puts a vehicle at `point` (absolute OMSI coordinates), at rest — [`respawn`] for any place
/// (the menu's *jump to entry point*). The recorded spawn point stays as it was.
///
/// # Errors
/// The vehicle does not exist or is not in the world.
pub fn respawn_at(world: &mut World, id: VehicleId, point: SpawnPoint) -> Result<(), OpError> {
    let entity = alive_entity(world, id)?;
    let local = abs_to_local(world, point.x, point.y);
    let active = world.resource::<SessionVehicles>().active_id() == Some(id);
    let Some(mut c) = world.get_mut::<VehicleController>(entity) else {
        return Err(OpError::NotReady(format!(
            "session vehicle {id} has no simulation"
        )));
    };
    c.sim
        .teleport(local, (point.heading_deg as f32).to_radians());
    let ch = &mut c.sim.chassis;
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
