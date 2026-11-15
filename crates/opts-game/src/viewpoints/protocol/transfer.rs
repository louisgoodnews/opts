//! `camera.viewpoint.export` and `camera.viewpoint.import`: the viewpoints file in and out.

use bevy::prelude::*;
use opts_bevy::control::params::str_param;
use opts_bevy::control::state::Outcome;
use opts_bevy::control::PathPolicy;
use opts_bevy::ViewpointState;
use opts_cli::protocol::{code, RpcError};
use serde_json::{json, Value};

use crate::viewpoints::share::read_import;
use crate::viewpoints::Viewpoints;

pub(super) fn viewpoints_missing() -> RpcError {
    RpcError::new(code::NOT_READY, "the viewpoint store is not available yet")
}

pub(super) fn export_inner(world: &mut World, params: &Value) -> Result<Value, RpcError> {
    let raw = str_param(params, "path")?;
    let path = world
        .resource::<PathPolicy>()
        .check_write(std::path::Path::new(raw))?;
    let viewpoints = world
        .get_resource::<Viewpoints>()
        .ok_or_else(viewpoints_missing)?;
    viewpoints.export_to(&path).map_err(|e| {
        RpcError::new(
            code::FAILED,
            format!("cannot write {}: {e}", path.display()),
        )
    })?;
    let vehicles = viewpoints
        .store
        .vehicles
        .values()
        .map(|c| c.len())
        .sum::<usize>();
    let models = viewpoints
        .store
        .models
        .values()
        .map(|c| c.len())
        .sum::<usize>();
    Ok(json!({ "file": path.display().to_string(), "vehicles": vehicles, "models": models }))
}

pub(super) fn export(world: &mut World, params: &Value) -> Outcome {
    match export_inner(world, params) {
        Ok(v) => Outcome::ok(v),
        Err(e) => Outcome::err(e),
    }
}

pub(super) fn import_inner(world: &mut World, params: &Value) -> Result<Value, RpcError> {
    let raw = str_param(params, "path")?;
    if world.resource::<ViewpointState>().editing {
        return Err(RpcError::new(
            code::GATED,
            "the viewpoint editor is open: save or cancel it first",
        ));
    }
    let path = world
        .resource::<PathPolicy>()
        .check_read(std::path::Path::new(raw))?;
    let (incoming, problems) = read_import(&path).map_err(RpcError::invalid_params)?;
    let mut summary = world
        .get_resource_mut::<Viewpoints>()
        .ok_or_else(viewpoints_missing)?
        .import(&incoming)
        .map_err(|e| {
            RpcError::new(
                code::FAILED,
                format!("cannot write the viewpoints file: {e}"),
            )
        })?;
    summary.problems = problems;
    // The vehicle in the driver's seat shows an imported value from the next frame on.
    world.resource_mut::<ViewpointState>().loaded_for = None;
    Ok(json!({
        "file": path.display().to_string(),
        "vehicles": summary.vehicles,
        "models": summary.models,
        "replaced": summary.replaced,
        "problems": summary.problems,
    }))
}

pub(super) fn import(world: &mut World, params: &Value) -> Outcome {
    match import_inner(world, params) {
        Ok(v) => Outcome::ok(v),
        Err(e) => Outcome::err(e),
    }
}
