//! `camera.viewpoint.set`: edits the offsets and the field of view of the open viewpoint.

use bevy::prelude::*;
use opts_bevy::camera::viewpoint::{FOV_RANGE_DEG, OFFSET_LIMIT_M};
use opts_bevy::control::params::opt_f64;
use opts_bevy::control::state::Outcome;
use opts_bevy::{CameraState, ViewpointState};
use opts_cli::protocol::{code, RpcError};
use serde_json::Value;

use super::view::{describe, modal_open, require_view};

pub(super) fn offset_param(params: &Value, name: &str) -> Result<Option<f32>, RpcError> {
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

pub(super) fn set_inner(world: &mut World, params: &Value) -> Result<Value, RpcError> {
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

pub(super) fn set(world: &mut World, params: &Value) -> Outcome {
    match set_inner(world, params) {
        Ok(v) => Outcome::ok(v),
        Err(e) => Outcome::err(e),
    }
}
