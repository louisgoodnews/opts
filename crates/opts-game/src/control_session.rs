//! Control-protocol methods of the session vehicles: `session.vehicles`, `session.spawn`,
//! `session.switch`, `session.despawn`, `session.respawn` (`docs/src/architecture/session-vehicles.md`).

use std::path::PathBuf;

use bevy::prelude::*;
use opts_bevy::control::params::{opt_bool, opt_str, opt_u64};
use opts_bevy::control::state::{ControlRegistry, Outcome};
use opts_bevy::control::PathPolicy;
use opts_cli::protocol::{code, RpcError};
use serde_json::{json, Value};

use crate::in_game::omsi_world::WorldLoadJob;
use crate::in_game::types::SimulationLoadState;
use crate::new_game_screen::types::GameCatalogue;
use crate::session_vehicles::ops::{self, OpError};
use crate::session_vehicles::{
    DespawnReason, EntrySel, PlaceLabel, SessionVehicles, SpawnParams, VehicleId,
};

/// Registers the methods.
pub fn register(registry: &mut ControlRegistry) {
    registry.register("session.vehicles", vehicles);
    registry.register("session.spawn", spawn);
    registry.register("session.switch", switch);
    registry.register("session.despawn", despawn);
    registry.register("session.respawn", respawn);
    registry.register("session.change_map", change_map);
}

fn op_error(e: OpError) -> RpcError {
    match e {
        OpError::NotFound(m) => RpcError::new(code::NOT_FOUND, m),
        OpError::NotReady(m) => RpcError::new(code::NOT_READY, m),
    }
}

fn session_up(world: &World) -> Result<(), RpcError> {
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

fn id_param(params: &Value) -> Result<Option<VehicleId>, RpcError> {
    opt_u64(params, "id")?
        .map(|n| {
            VehicleId::try_from(n)
                .map_err(|_| RpcError::invalid_params("`id` is not a session vehicle id"))
        })
        .transpose()
}

fn active_or(world: &World, id: Option<VehicleId>) -> Result<VehicleId, RpcError> {
    id.or_else(|| world.resource::<SessionVehicles>().active_id())
        .ok_or_else(|| RpcError::new(code::NOT_READY, "there is no active session vehicle"))
}

fn vehicles(world: &mut World, _: &Value) -> Outcome {
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

/// `session.change_map {map, entry?, vehicle?, dry_run?}`: the Alt menu's *Change map*.
fn change_map(world: &mut World, params: &Value) -> Outcome {
    use crate::in_game::change_map::{plan_in_world, start, ChangeMapError, VehicleChoice};
    let result = (|| -> Result<Value, RpcError> {
        let map = opts_bevy::control::params::str_param(params, "map")?;
        let entry = opt_str(params, "entry")?;
        let choice = match opt_str(params, "vehicle")? {
            None | Some("carry") => VehicleChoice::Carry,
            Some("none") => VehicleChoice::None,
            Some(_) => {
                return Err(RpcError::invalid_params("`vehicle` is `carry` or `none`"));
            }
        };
        let dry_run = opt_bool(params, "dry_run")?.unwrap_or(false);
        let plan = plan_in_world(world, map, entry, choice).map_err(|e| match e {
            ChangeMapError::NotInDrive | ChangeMapError::NoCatalogue => {
                RpcError::new(code::NOT_READY, e.to_string())
            }
            ChangeMapError::VehicleUnavailable(_) => RpcError::new(code::NOT_FOUND, e.to_string()),
            _ => RpcError::invalid_params(e.to_string()),
        })?;
        let mut answer = plan.describe();
        answer["started"] = json!(!dry_run);
        if !dry_run {
            start(world, &plan);
        }
        Ok(answer)
    })();
    Outcome::Done(result)
}

/// The entry point a spawn asks for: `entry_id`, else `entry` (a name), else the session's own.
pub(crate) fn entry_param(params: &Value) -> Result<EntrySel, RpcError> {
    Ok(
        match (opt_u64(params, "entry_id")?, opt_str(params, "entry")?) {
            (Some(i), _) => EntrySel::Id(
                u32::try_from(i).map_err(|_| RpcError::invalid_params("`entry_id` too large"))?,
            ),
            (None, Some(n)) => EntrySel::Name(n.to_owned()),
            (None, None) => EntrySel::Default,
        },
    )
}

pub(crate) fn session_ready(world: &World) -> Result<(), RpcError> {
    session_up(world)
}

fn spawn(world: &mut World, params: &Value) -> Outcome {
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
        let entry = entry_param(params)?;
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

fn switch(world: &mut World, params: &Value) -> Outcome {
    let result = (|| -> Result<Value, RpcError> {
        session_up(world)?;
        let id = match (
            id_param(params)?,
            params.get("step").and_then(Value::as_i64),
        ) {
            (Some(id), _) => {
                ops::switch_to(world, id).map_err(op_error)?;
                id
            }
            (None, Some(step)) => ops::switch_step(world, step as i32).map_err(op_error)?,
            (None, None) => {
                return Err(RpcError::invalid_params("give `id` or `step`"));
            }
        };
        Ok(json!({ "ok": true, "active": id }))
    })();
    result.into()
}

fn despawn(world: &mut World, params: &Value) -> Outcome {
    let result = (|| -> Result<Value, RpcError> {
        session_up(world)?;
        let id = active_or(world, id_param(params)?)?;
        let out = ops::despawn(world, id, DespawnReason::User).map_err(op_error)?;
        Ok(json!({ "ok": true, "active": out.active, "removed": out.removed }))
    })();
    result.into()
}

fn respawn(world: &mut World, params: &Value) -> Outcome {
    let result = (|| -> Result<Value, RpcError> {
        session_up(world)?;
        let id = active_or(world, id_param(params)?)?;
        ops::respawn(world, id).map_err(op_error)?;
        Ok(json!({ "ok": true, "id": id }))
    })();
    result.into()
}
