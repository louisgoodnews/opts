//! Control-protocol methods of the session save: `session.save`, `session.load`, `session.saves`,
//! `session.delete_save`, `session.rename_save` (`docs/src/architecture/session-save.md`).

mod save_load;
mod saves;

use save_load::{load, save};
use saves::saves;

use bevy::prelude::*;
use opts_bevy::control::params::{opt_str, str_param};
use opts_bevy::control::state::{ControlRegistry, Outcome};
use opts_cli::protocol::{code, RpcError};
use serde_json::{json, Value};

use crate::save::container::SaveError;
use crate::save::ops;
use crate::save::restore::RestoreError;

/// Registers the methods.
pub fn register(registry: &mut ControlRegistry) {
    registry.register("session.save", save);
    registry.register("session.load", load);
    registry.register("session.saves", saves);
    registry.register("session.delete_save", delete_save);
    registry.register("session.rename_save", rename_save);
}

fn save_error(e: &SaveError) -> RpcError {
    let c = match e {
        SaveError::NotFound(_) => code::NOT_FOUND,
        SaveError::NoSession(_) => code::NOT_READY,
        SaveError::InvalidName(_) => code::INVALID_PARAMS,
        _ => code::FAILED,
    };
    RpcError::new(c, e.to_string()).with_data(json!({ "kind": e.kind() }))
}

fn restore_error(e: &RestoreError) -> RpcError {
    match e {
        RestoreError::Save(s) => save_error(s),
        RestoreError::Blocked(issues) => {
            RpcError::new(code::FAILED, e.to_string()).with_data(json!({
                "kind": e.kind(),
                "issues": issues.iter().map(|i| json!({
                    "kind": format!("{:?}", i.kind),
                    "subject": i.subject,
                    "blocking": i.blocking,
                })).collect::<Vec<_>>(),
            }))
        }
    }
}

pub(super) fn resolve(world: &World, params: &Value) -> Result<String, RpcError> {
    let text = str_param(params, "name")?;
    let store = ops::store(world).map_err(|e| save_error(&e))?;
    store
        .find(text)
        .ok_or_else(|| save_error(&SaveError::NotFound(text.to_owned())))
}

fn delete_save(world: &mut World, params: &Value) -> Outcome {
    let result = (|| {
        let id = resolve(world, params)?;
        ops::store(world)
            .and_then(|s| s.delete(&id))
            .map_err(|e| save_error(&e))?;
        Ok::<_, RpcError>(json!({ "ok": true, "id": id }))
    })();
    result.into()
}

fn rename_save(world: &mut World, params: &Value) -> Outcome {
    let result = (|| {
        let id = resolve(world, params)?;
        let new_name = opt_str(params, "new_name")?
            .ok_or_else(|| RpcError::invalid_params("missing `new_name`"))?;
        ops::store(world)
            .and_then(|s| s.rename(&id, new_name))
            .map_err(|e| save_error(&e))?;
        Ok::<_, RpcError>(json!({ "ok": true, "id": id, "name": new_name.trim() }))
    })();
    result.into()
}
