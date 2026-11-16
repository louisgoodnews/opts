//! Listing and spawning of the session vehicles: `session.vehicles` and `session.spawn`, and
//! the `.bus` file a spawn names.

use std::path::PathBuf;

use bevy::prelude::*;
use opts_bevy::control::params::opt_str;
use opts_bevy::control::state::Outcome;
use opts_bevy::control::PathPolicy;
use opts_cli::protocol::{code, RpcError};
use serde_json::{json, Value};

use crate::in_game::omsi_world::WorldLoadJob;
use crate::new_game_screen::types::GameCatalogue;
use crate::session_vehicles::{EntrySel, PlaceLabel, SessionVehicles, SpawnParams};

use super::{entry_param, session_up, stop_param};

pub(super) fn vehicles(world: &mut World, _: &Value) -> Outcome {
    if !world.contains_resource::<SessionVehicles>() {
        return Outcome::err(RpcError::new(
            code::NOT_READY,
            "no session has been started",
        ));
    }
    let mut out = world.resource::<SessionVehicles>().describe();
    let label = world.get_resource::<PlaceLabel>().and_then(|p| p.0.clone());
    out["place_label"] = json!(label);
    Outcome::ok(out)
}

/// The `.bus` file a `vehicle` parameter names, with the catalogue entry it came from.
fn resolve_vehicle(world: &World, text: &str) -> Result<(String, String, PathBuf), RpcError> {
    let as_path = PathBuf::from(text);
    let looks_like_file = text.to_lowercase().ends_with(".bus") || as_path.is_absolute();
    if looks_like_file {
        let checked = world.resource::<PathPolicy>().check_read(&as_path)?;
        if !checked.is_file() {
            return Err(RpcError::new(
                code::NOT_FOUND,
                format!("{} is not a file", checked.display()),
            ));
        }
        let stem = checked
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        return Ok((stem.clone(), stem, checked));
    }
    let item = world
        .get_resource::<GameCatalogue>()
        .and_then(|c| c.find_vehicle(text).cloned())
        .ok_or_else(|| {
            RpcError::new(
                code::NOT_FOUND,
                format!("no vehicle `{text}` in the catalogue (or pass an absolute .bus path)"),
            )
        })?;
    let file = world
        .get_resource::<WorldLoadJob>()
        .and_then(|j| j.bus_file(&item.bus_file))
        .ok_or_else(|| {
            RpcError::new(
                code::NOT_FOUND,
                format!("the .bus file of `{}` was not found", item.full_name),
            )
        })?;
    Ok((item.id, item.full_name, file))
}

pub(super) fn spawn(world: &mut World, params: &Value) -> Outcome {
    let result = (|| -> Result<Value, RpcError> {
        session_up(world)?;
        let text = opts_bevy::control::params::str_param(params, "vehicle")?;
        let (catalogue_id, name, file) = resolve_vehicle(world, text)?;
        let setvars = match params.get("setvars") {
            None | Some(Value::Null) => Vec::new(),
            Some(Value::Object(map)) => {
                let mut v = Vec::new();
                for (k, val) in map {
                    let n = val
                        .as_i64()
                        .or_else(|| val.as_bool().map(i64::from))
                        .ok_or_else(|| {
                            RpcError::invalid_params(format!("setvars.{k} must be 0, 1 or a bool"))
                        })?;
                    v.push((k.clone(), n as i32));
                }
                v.sort();
                v
            }
            Some(_) => {
                return Err(RpcError::invalid_params(
                    "`setvars` must be an object {variable: value}",
                ))
            }
        };
        let entry = match stop_param(world, params)? {
            Some(station) => EntrySel::Stop(station),
            None => entry_param(params)?,
        };
        let id = world
            .resource_mut::<SessionVehicles>()
            .request_spawn(SpawnParams {
                catalogue_id,
                name,
                bus_file: file.display().to_string(),
                repaint: opt_str(params, "repaint")?.unwrap_or_default().to_owned(),
                setvars,
                entry,
                formation: None,
            });
        Ok(json!({ "id": id, "queued": true }))
    })();
    result.into()
}
