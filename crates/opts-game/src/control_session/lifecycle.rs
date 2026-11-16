//! Changing and ending the session vehicle's place: `session.change_map`, `session.switch`,
//! `session.despawn`, `session.respawn` and `session.reset_road`.

use bevy::prelude::*;
use opts_bevy::control::params::{opt_bool, opt_str};
use opts_bevy::control::state::Outcome;
use opts_cli::protocol::{code, RpcError};
use serde_json::{json, Value};

use crate::session_vehicles::ops;
use crate::session_vehicles::DespawnReason;

use super::{active_or, id_param, op_error, session_up};

/// `session.change_map {map, entry?, vehicle?, dry_run?}`: the Alt menu's *Change map*.
pub(super) fn change_map(world: &mut World, params: &Value) -> Outcome {
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

pub(super) fn switch(world: &mut World, params: &Value) -> Outcome {
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

pub(super) fn despawn(world: &mut World, params: &Value) -> Outcome {
    let result = (|| -> Result<Value, RpcError> {
        session_up(world)?;
        let id = active_or(world, id_param(params)?)?;
        let out = ops::despawn(world, id, DespawnReason::User).map_err(op_error)?;
        Ok(json!({ "ok": true, "active": out.active, "removed": out.removed }))
    })();
    result.into()
}

pub(super) fn respawn(world: &mut World, params: &Value) -> Outcome {
    let result = (|| -> Result<Value, RpcError> {
        session_up(world)?;
        let id = active_or(world, id_param(params)?)?;
        ops::respawn(world, id).map_err(op_error)?;
        Ok(json!({ "ok": true, "id": id }))
    })();
    result.into()
}

pub(super) fn reset_road(world: &mut World, params: &Value) -> Outcome {
    let result = (|| -> Result<Value, RpcError> {
        session_up(world)?;
        let id = active_or(world, id_param(params)?)?;
        crate::session_vehicles::reset_on_road(world, id).map_err(op_error)?;
        Ok(json!({ "ok": true, "id": id }))
    })();
    result.into()
}
