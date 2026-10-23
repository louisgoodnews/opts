//! Control-protocol methods of the viewpoint editor: `camera.viewpoint.get`, `.set`, `.save` and
//! `.reset`. They work on the same [`ViewpointState`] as the keys and the panel, so a scripted run
//! sees exactly what a player gets.

use bevy::prelude::*;
use opts_bevy::camera::viewpoint::{FOV_RANGE_DEG, OFFSET_LIMIT_M};
use opts_bevy::control::params::opt_f64;
use opts_bevy::control::state::{ControlRegistry, Outcome};
use opts_bevy::{BusVehicle, CameraMode, CameraState, PlayerVehicle, Viewpoint, ViewpointState};
use opts_cli::protocol::{code, RpcError};
use serde_json::{json, Value};

use super::{persist_viewpoint, Viewpoints};

/// Registers the methods.
pub fn register(registry: &mut ControlRegistry) {
    registry.register("camera.viewpoint.get", get);
    registry.register("camera.viewpoint.set", set);
    registry.register("camera.viewpoint.save", save);
    registry.register("camera.viewpoint.reset", reset);
}

fn json_of(vp: &Viewpoint) -> Value {
    super::vp_json(vp)
}

fn describe(world: &mut World) -> Value {
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
fn require_view(world: &World) -> Result<(), RpcError> {
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

fn modal_open(world: &World) -> bool {
    world
        .get_resource::<opts_bevy::reporting::state::ModalActive>()
        .is_some_and(|m| m.0)
        || world
            .get_resource::<opts_bevy::reporting::state::MenuModalActive>()
            .is_some_and(|m| m.0)
}

fn get(world: &mut World, _: &Value) -> Outcome {
    Outcome::ok(describe(world))
}

fn offset_param(params: &Value, name: &str) -> Result<Option<f32>, RpcError> {
    let Some(v) = opt_f64(params, name)? else {
        return Ok(None);
    };
    if !v.is_finite() || v.abs() > f64::from(OFFSET_LIMIT_M) + 1e-6 {
        return Err(RpcError::new(
            code::INVALID_PARAMS,
            format!("`{name}` is {v} m; the limit is +-{OFFSET_LIMIT_M} m"),
        ));
    }
    Ok(Some(v as f32))
}

fn set_inner(world: &mut World, params: &Value) -> Result<Value, RpcError> {
    let (x, y, z) = (
        offset_param(params, "x")?,
        offset_param(params, "y")?,
        offset_param(params, "z")?,
    );
    let fov = opt_f64(params, "fov_deg")?;
    if let Some(f) = fov {
        if !f.is_finite() || f < f64::from(FOV_RANGE_DEG.0) || f > f64::from(FOV_RANGE_DEG.1) {
            return Err(RpcError::new(
                code::INVALID_PARAMS,
                format!(
                    "`fov_deg` is {f}; the range is {}..{} degrees",
                    FOV_RANGE_DEG.0, FOV_RANGE_DEG.1
                ),
            ));
        }
    }
    if x.is_none() && y.is_none() && z.is_none() && fov.is_none() {
        return Err(RpcError::new(
            code::INVALID_PARAMS,
            "give at least one of x, y, z (metres) or fov_deg (degrees)",
        ));
    }
    if modal_open(world) {
        return Err(RpcError::new(
            code::GATED,
            "a dialog or the vehicle menu is open: camera input is blocked",
        ));
    }
    require_view(world)?;
    let index = world.resource::<CameraState>().sub_camera_index;
    let mut state = world.resource_mut::<ViewpointState>();
    if !state.editing {
        state.camera_index = index;
        state.open();
    }
    let mut vp = state.active;
    vp.x = x.unwrap_or(vp.x);
    vp.y = y.unwrap_or(vp.y);
    vp.z = z.unwrap_or(vp.z);
    if let Some(f) = fov {
        vp.fov_deg = Some(f as f32);
    }
    state.set(vp);
    Ok(describe(world))
}

fn set(world: &mut World, params: &Value) -> Outcome {
    match set_inner(world, params) {
        Ok(v) => Outcome::ok(v),
        Err(e) => Outcome::err(e),
    }
}

fn save(world: &mut World, params: &Value) -> Outcome {
    if let Err(e) = require_view(world) {
        return Outcome::err(e);
    }
    let scope = match opts_bevy::control::params::opt_str(params, "scope") {
        Ok(s) => s.unwrap_or("vehicle"),
        Err(e) => return Outcome::err(e),
    };
    if !matches!(scope, "vehicle" | "model") {
        return Outcome::err(RpcError::new(
            code::INVALID_PARAMS,
            "`scope` is `vehicle` or `model`",
        ));
    }
    let mut state = world.resource_mut::<ViewpointState>();
    if scope == "model" && state.model_key.is_none() {
        return Outcome::err(RpcError::new(
            code::NOT_READY,
            "the vehicle's model is not known: there is nothing to apply the viewpoint to",
        ));
    }
    state.scope_model = scope == "model";
    state.save();
    persist_viewpoint(world);
    Outcome::ok(describe(world))
}

fn reset(world: &mut World, _: &Value) -> Outcome {
    if let Err(e) = require_view(world) {
        return Outcome::err(e);
    }
    let index = world.resource::<CameraState>().sub_camera_index;
    let mut state = world.resource_mut::<ViewpointState>();
    state.camera_index = index;
    state.reset();
    state.save();
    persist_viewpoint(world);
    Outcome::ok(describe(world))
}
