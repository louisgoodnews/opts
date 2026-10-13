//! Control-protocol methods of the session save: `session.save`, `session.load`, `session.saves`,
//! `session.delete_save`, `session.rename_save` (`docs/src/architecture/session-save.md`).

use bevy::prelude::*;
use opts_bevy::control::params::{opt_bool, opt_str, str_param};
use opts_bevy::control::state::{ControlRegistry, Outcome};
use opts_cli::protocol::{code, RpcError};
use serde_json::{json, Value};

use crate::save::container::SaveError;
use crate::save::ops::{self, SaveRequest};
use crate::save::registry::{preflight, CheckEnv, SaveRegistry};
use crate::save::restore::{begin_restore, LastRestore, PendingRestore, RestoreError, RestoreOutcome};

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
        RestoreError::Blocked(issues) => RpcError::new(code::FAILED, e.to_string()).with_data(json!({
            "kind": e.kind(),
            "issues": issues.iter().map(|i| json!({
                "kind": format!("{:?}", i.kind),
                "subject": i.subject,
                "blocking": i.blocking,
            })).collect::<Vec<_>>(),
        })),
    }
}

fn save(world: &mut World, params: &Value) -> Outcome {
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
                format!("a save called `{name}` exists (`{id}`); pass overwrite=true to replace it"),
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

fn resolve(world: &World, params: &Value) -> Result<String, RpcError> {
    let text = str_param(params, "name")?;
    let store = ops::store(world).map_err(|e| save_error(&e))?;
    store
        .find(text)
        .ok_or_else(|| save_error(&SaveError::NotFound(text.to_owned())))
}

fn load(world: &mut World, params: &Value) -> Outcome {
    let result = (|| {
        if world.contains_resource::<PendingRestore>() {
            return Err(RpcError::new(code::BUSY, "a save is being restored already"));
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

fn restore_status(world: &World) -> Value {
    if world.contains_resource::<PendingRestore>() {
        return json!({ "state": "loading" });
    }
    match world.get_resource::<LastRestore>().and_then(|l| l.0.clone()) {
        Some(RestoreOutcome::Done { id, warnings }) => {
            json!({ "state": "done", "id": id, "warnings": warnings })
        }
        Some(RestoreOutcome::Failed { id }) => json!({ "state": "failed", "id": id }),
        None => json!({ "state": "idle" }),
    }
}

fn saves(world: &mut World, _: &Value) -> Outcome {
    let result = (|| {
        let store = ops::store(world).map_err(|e| save_error(&e))?;
        let registry = world.resource::<SaveRegistry>().clone();
        let root = world
            .get_resource::<crate::in_game::omsi_world::WorldLoadJob>()
            .map(|j| j.config.root.clone())
            .or_else(crate::omsi_root::known_omsi_root);
        let list: Vec<Value> = store
            .list()
            .iter()
            .map(|entry| match &entry.state {
                Ok(l) => {
                    let f = &l.file;
                    let issues = preflight(
                        registry.all(),
                        f,
                        &CheckEnv {
                            root: root.as_deref(),
                            fingerprint: &f.fingerprint,
                        },
                    );
                    json!({
                        "id": entry.id,
                        "name": f.meta.name,
                        "map": f.meta.map,
                        "vehicles": f.meta.vehicles,
                        "play_time_s": f.meta.play_time_s,
                        "created_unix": f.meta.created_unix,
                        "saved_unix": f.meta.saved_unix,
                        "format_version": f.format_version,
                        "opts_version": f.opts_version,
                        "sections": f.sections.iter().map(|(k, v)| json!({"id": k, "version": v.version})).collect::<Vec<_>>(),
                        "unknown_sections": f.sections.keys().filter(|k| registry.get(k).is_none()).collect::<Vec<_>>(),
                        "from_backup": l.from_backup,
                        "issues": issues.iter().map(|i| json!({
                            "kind": format!("{:?}", i.kind),
                            "subject": i.subject,
                            "blocking": i.blocking,
                        })).collect::<Vec<_>>(),
                    })
                }
                Err(e) => json!({
                    "id": entry.id,
                    "error": { "kind": e.kind(), "message": e.to_string() },
                }),
            })
            .collect();
        Ok::<_, RpcError>(json!({
            "dir": store.dir().display().to_string(),
            "saves": list,
            "sections": registry
                .all()
                .iter()
                .map(|s| json!({ "id": s.id, "version": s.version, "about": s.about }))
                .collect::<Vec<_>>(),
            "restore": restore_status(world),
            "running": ops::session_running(world),
        }))
    })();
    result.into()
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
