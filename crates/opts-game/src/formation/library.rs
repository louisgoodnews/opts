//! The control-protocol methods on the saved formations: `formation.save`, `.list` and `.delete`.

use bevy::prelude::*;
use opts_bevy::control::params::opt_str;
use opts_bevy::control::state::Outcome;
use opts_cli::protocol::{code, RpcError};
use serde_json::{json, Value};

use super::request::{formation_from, store_error};
use super::store::{self, FormationStore};
use super::FormationLibrary;

fn library(world: &World) -> Result<FormationStore, RpcError> {
    world
        .get_resource::<FormationLibrary>()
        .and_then(|l| l.0.clone())
        .ok_or_else(|| RpcError::new(code::NOT_READY, "no user data folder for formations"))
}

/// `formation.save`: stores the formation the call describes under a new id made from its name
/// (or under `id`, replacing that file, when `overwrite` is true).
pub(super) fn save(world: &mut World, params: &Value) -> Outcome {
    let result = (|| -> Result<Value, RpcError> {
        let mut formation = formation_from(world, params)?;
        if formation.is_empty() {
            return Err(RpcError::invalid_params("an empty formation is not saved"));
        }
        if let Some(name) = opt_str(params, "name")? {
            formation.name = name.to_owned();
        }
        let store = library(world)?;
        let id = match (
            opt_str(params, "id")?,
            params.get("overwrite").and_then(Value::as_bool) == Some(true),
        ) {
            (Some(id), true) => {
                store.write(id, &formation).map_err(store_error)?;
                id.to_owned()
            }
            (Some(_), false) => {
                return Err(RpcError::invalid_params("`id` needs `overwrite: true`"))
            }
            (None, _) => store.save_new(&formation).map_err(store_error)?,
        };
        Ok(json!({ "ok": true, "id": id, "name": formation.name, "cars": formation.len() }))
    })();
    result.into()
}

/// One row of `formation.list`.
pub(super) fn saved_json(s: &store::SavedFormation) -> Value {
    let modified = s
        .modified
        .and_then(|m| m.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs());
    match &s.state {
        Ok(f) => json!({ "id": s.id, "name": f.name, "cars": f.len(), "modified": modified }),
        Err(e) => json!({ "id": s.id, "error": e.to_string(), "modified": modified }),
    }
}

pub(super) fn list(world: &mut World, _: &Value) -> Outcome {
    let result = (|| -> Result<Value, RpcError> {
        let store = library(world)?;
        let saved: Vec<Value> = store.list().iter().map(saved_json).collect();
        Ok(json!({ "dir": store.dir().display().to_string(), "formations": saved }))
    })();
    result.into()
}

pub(super) fn delete(world: &mut World, params: &Value) -> Outcome {
    let result = (|| -> Result<Value, RpcError> {
        let id = opts_bevy::control::params::str_param(params, "id")?;
        library(world)?.delete(id).map_err(store_error)?;
        Ok(json!({ "ok": true, "id": id }))
    })();
    result.into()
}
