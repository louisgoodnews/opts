//! The state the methods report: the viewpoint, the stored one and the limits.

use bevy::prelude::*;
use opts_bevy::camera::viewpoint::{FOV_RANGE_DEG, OFFSET_LIMIT_M};
use opts_bevy::{BusVehicle, CameraMode, CameraState, PlayerVehicle, Viewpoint, ViewpointState};
use opts_cli::protocol::{code, RpcError};
use serde_json::{json, Value};

use crate::viewpoints::Viewpoints;

pub(super) fn json_of(vp: &Viewpoint) -> Value {
    crate::viewpoints::vp_json(vp)
}

pub(super) fn describe(world: &mut World) -> Value {
    let state = world.resource::<ViewpointState>().clone();
    let total = world
        .query_filtered::<&BusVehicle, With<PlayerVehicle>>()
        .iter(world)
        .next()
        .map_or(0, |b| b.driver_camera_count());
    let path = world
        .get_resource::<Viewpoints>()
        .and_then(|v| v.path.as_ref().map(|p| p.display().to_string()));
    json!({
        "editing": state.editing,
        "dirty": state.is_dirty(),
        "row": state.row.id(),
        "vehicle": state.vehicle_key,
        "model": state.model_key,
        "scope": if state.scope_model { "model" } else { "vehicle" },
        "user_default_fov_deg": state.user_default_fov_deg,
        "camera": state.camera_index,
        "cameras": total,
        "viewpoint": json_of(&state.active),
        "stored": json_of(&state.committed),
        "fov_deg": state.effective_fov_deg(),
        "default_fov_deg": state.default_fov_deg,
        "limits": {
            "offset_m": (f64::from(OFFSET_LIMIT_M) * 100.0).round() / 100.0,
            "fov_deg": [FOV_RANGE_DEG.0, FOV_RANGE_DEG.1],
        },
        "file": path,
    })
}

/// The driver's view must be on screen and have a vehicle with a known identity.
pub(super) fn require_view(world: &World) -> Result<(), RpcError> {
    let mode = world.get_resource::<CameraState>().map(|c| c.mode);
    if mode != Some(CameraMode::Cockpit) {
        return Err(RpcError::new(
            code::NOT_READY,
            "the viewpoint belongs to the driver's view: switch with `camera.set_mode cockpit`",
        ));
    }
    if world.resource::<ViewpointState>().vehicle_key.is_none() {
        return Err(RpcError::new(
            code::NOT_READY,
            "the player's vehicle has no catalogue identity yet",
        ));
    }
    Ok(())
}

pub(super) fn modal_open(world: &World) -> bool {
    world
        .get_resource::<opts_bevy::reporting::state::ModalActive>()
        .is_some_and(|m| m.0)
        || world
            .get_resource::<opts_bevy::reporting::state::MenuModalActive>()
            .is_some_and(|m| m.0)
}
