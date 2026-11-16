//! Reading the formation a control call describes, and the parameter and store error helpers.

use bevy::prelude::*;
use opts_bevy::control::params::opt_str;
use opts_cli::protocol::{code, RpcError};
use opts_core::formats::trains::{
    FormationSegment, FormationSource, Orientation, TrainFormation, VehicleRef,
};
use serde_json::Value;

use crate::in_game::omsi_world::WorldLoadJob;
use crate::in_game::types::SimulationLoadState;
use crate::session_vehicles::SessionVehicles;

use super::store::StoreError;
use super::FormationLibrary;

pub(super) fn session_up(world: &World) -> Result<(), RpcError> {
    if !world.contains_resource::<SessionVehicles>() {
        return Err(RpcError::new(
            code::NOT_READY,
            "no session has been started",
        ));
    }
    if world
        .get_resource::<SimulationLoadState>()
        .is_some_and(|s| !s.is_complete)
    {
        return Err(RpcError::new(
            code::NOT_READY,
            "the session is still loading",
        ));
    }
    Ok(())
}

pub(super) fn orientation_of(text: Option<&str>) -> Result<Orientation, RpcError> {
    match text.map(str::to_ascii_lowercase).as_deref() {
        None | Some("forward" | "0") => Ok(Orientation::Forward),
        Some("backward" | "1") => Ok(Orientation::Backward),
        Some(o) => Err(RpcError::invalid_params(format!(
            "orientation `{o}` is not `forward` or `backward`"
        ))),
    }
}

/// The formation a call describes: a `.zug` of the installation (`zug`), or `cars`, an array of
/// vehicle files or `{vehicle, orientation}` objects. `reverse` turns the train around.
pub(super) fn formation_from(world: &World, params: &Value) -> Result<TrainFormation, RpcError> {
    let mut formation = if let Some(id) = opt_str(params, "saved")? {
        let library = world.get_resource::<FormationLibrary>();
        let store = library
            .and_then(|l| l.0.as_ref())
            .ok_or_else(|| RpcError::new(code::NOT_READY, "no user data folder for formations"))?;
        store.read(id).map_err(store_error)?
    } else if let Some(zug) = opt_str(params, "zug")? {
        let job = world
            .get_resource::<WorldLoadJob>()
            .ok_or_else(|| RpcError::new(code::NOT_READY, "no session has been started"))?;
        let path = job.bus_file(zug).ok_or_else(|| {
            RpcError::new(
                code::NOT_FOUND,
                format!("`{zug}` is not in the installation"),
            )
        })?;
        let bytes = std::fs::read(&path)
            .map_err(|e| RpcError::new(code::NOT_FOUND, format!("{}: {e}", path.display())))?;
        let name = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        TrainFormation::from_zug(&bytes, &name, zug)
            .map_err(|e| RpcError::invalid_params(format!("{zug}: {e}")))?
            .0
    } else if let Some(cars) = params.get("cars") {
        let Value::Array(items) = cars else {
            return Err(RpcError::invalid_params("`cars` must be an array"));
        };
        let mut segments = Vec::with_capacity(items.len());
        for (i, item) in items.iter().enumerate() {
            let (vehicle, orientation, powered) = match item {
                Value::String(s) => (s.as_str(), None, None),
                Value::Object(o) => (
                    o.get("vehicle").and_then(Value::as_str).ok_or_else(|| {
                        RpcError::invalid_params(format!("cars[{i}] needs a `vehicle`"))
                    })?,
                    o.get("orientation").and_then(Value::as_str),
                    o.get("powered").and_then(Value::as_bool),
                ),
                _ => {
                    return Err(RpcError::invalid_params(format!(
                        "cars[{i}] must be a string or an object"
                    )))
                }
            };
            segments.push(FormationSegment {
                vehicle: VehicleRef::from_entry(vehicle),
                orientation: orientation_of(orientation)?,
                powered,
            });
        }
        TrainFormation {
            name: opt_str(params, "name")?.unwrap_or("Formation").to_owned(),
            segments,
            source: FormationSource::Custom,
        }
    } else {
        return Err(RpcError::invalid_params("give `saved`, `zug` or `cars`"));
    };
    if params.get("reverse").and_then(Value::as_bool) == Some(true) {
        formation = formation.reversed();
    }
    Ok(formation)
}

pub(super) fn store_error(e: StoreError) -> RpcError {
    match e {
        StoreError::NotFound(_) => RpcError::new(code::NOT_FOUND, e.to_string()),
        StoreError::InvalidName(_) => RpcError::invalid_params(e.to_string()),
        _ => RpcError::new(code::FAILED, e.to_string()),
    }
}
