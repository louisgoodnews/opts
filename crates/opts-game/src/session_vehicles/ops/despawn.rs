//! Despawning a session vehicle with its trailer chain and parts, and the successor rule.

use bevy::prelude::*;
use opts_bevy::vehicle_remote::RemoteControls;
use opts_bevy::{ArticulatedChain, CameraMode, CameraState};

use crate::in_game::omsi_world::{VehicleComposition, WorldStats};
use crate::session_vehicles::registry::{DespawnReason, Lifecycle, SessionVehicles, VehicleId};

use super::frame::OpError;
use super::switch::switch_to;

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
        .query_filtered::<&Transform, With<opts_bevy::MainCamera>>()
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
