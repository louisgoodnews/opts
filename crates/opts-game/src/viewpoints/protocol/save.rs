//! `camera.viewpoint.save` and `camera.viewpoint.reset`: writes or removes the stored viewpoint.

use bevy::prelude::*;
use opts_bevy::control::state::Outcome;
use opts_bevy::{CameraState, ViewpointState};
use opts_cli::protocol::{code, RpcError};
use serde_json::Value;

use crate::viewpoints::persist_viewpoint;

use super::view::{describe, require_view};

pub(super) fn save(world: &mut World, params: &Value) -> Outcome {
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

pub(super) fn reset(world: &mut World, _: &Value) -> Outcome {
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
