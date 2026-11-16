//! Saving and loading the session (`session.save`, `session.load`) and the state of the last
//! restore they report.

use bevy::prelude::*;
use opts_bevy::control::params::{opt_bool, str_param};
use opts_bevy::control::state::Outcome;
use opts_cli::protocol::{code, RpcError};
use serde_json::{json, Value};

use crate::save::container::SaveError;
use crate::save::ops::{self, SaveRequest};
use crate::save::restore::{begin_restore, LastRestore, PendingRestore, RestoreOutcome};

use super::{resolve, restore_error, save_error};

pub(super) fn save(world: &mut World, params: &Value) -> Outcome {
    let result = (|| {
        let name = str_param(params, "name")?;
        let request = SaveRequest {
            name: name.to_owned(),
            id: None,
            overwrite: opt_bool(params, "overwrite")?.unwrap_or(false),
        };
        let saved = ops::save_session(world, &request).map_err(|e| match e {
            SaveError::Exists(id) => RpcError::new(
                code::FAILED,
                format!(
                    "a save called `{name}` exists (`{id}`); pass overwrite=true to replace it"
                ),
            )
            .with_data(json!({ "kind": "exists", "id": id })),
            other => save_error(&other),
        })?;
        Ok::<_, RpcError>(json!({
            "ok": true,
            "id": saved.id,
            "name": saved.name,
            "overwritten": saved.overwritten,
        }))
    })();
    result.into()
}

pub(super) fn load(world: &mut World, params: &Value) -> Outcome {
    let result = (|| {
        if world.contains_resource::<PendingRestore>() {
            return Err(RpcError::new(
                code::BUSY,
                "a save is being restored already",
            ));
        }
        let id = resolve(world, params)?;
        let started = begin_restore(world, &id).map_err(|e| restore_error(&e))?;
        Ok(json!({
            "ok": true,
            "started": true,
            "id": started.id,
            "name": started.name,
            "unknown_sections": started.unknown_sections,
            "missing_sections": started.missing_sections,
            "warnings": started.warnings,
        }))
    })();
    result.into()
}

pub(super) fn restore_status(world: &World) -> Value {
    if world.contains_resource::<PendingRestore>() {
        return json!({ "state": "loading" });
    }
    match world
        .get_resource::<LastRestore>()
        .and_then(|l| l.0.clone())
    {
        Some(RestoreOutcome::Done { id, warnings }) => {
            json!({ "state": "done", "id": id, "warnings": warnings })
        }
        Some(RestoreOutcome::Failed { id }) => json!({ "state": "failed", "id": id }),
        None => json!({ "state": "idle" }),
    }
}
