//! Switching the active session vehicle: the driver's input, the camera and the HUD follow it.

use bevy::prelude::*;
use opts_bevy::vehicle_remote::RemoteControls;
use opts_bevy::{CameraState, PlayerVehicle};

use crate::in_game::omsi_world::WorldStats;
use crate::session_vehicles::registry::{SessionVehicles, VehicleId};

use super::frame::{alive_entity, park, OpError};

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
    world.resource_mut::<SessionVehicles>().set_active(Some(id));
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
