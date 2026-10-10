//! `reload.*` and `watch.*`: explicit reloads and watch roots (`docs/src/architecture/hot-reload.md`).

use std::path::PathBuf;

use bevy::ecs::world::World;
use opts_cli::protocol::{code, RpcError};
use serde_json::{json, Value};

use crate::control::params::str_param;
use crate::control::paths::PathPolicy;
use crate::control::state::{ControlRegistry, Outcome};
use crate::hot_reload::{
    classify, FileWatcher, ReloadApplierRegistered, ReloadRequested, ReloadSceneRequested,
    ReloadStats,
};

/// Registers the reload methods.
pub fn register(r: &mut ControlRegistry) {
    r.register("reload.asset", reload_asset);
    r.register("reload.config", reload_asset);
    r.register("reload.scene", reload_scene);
    r.register("watch.add", watch_add);
    r.register("watch.remove", watch_remove);
    r.register("watch.list", watch_list);
}

fn not_ready() -> RpcError {
    RpcError::new(
        code::NOT_READY,
        "hot reload is not running in this application",
    )
}

/// Answers once request `seq` has an outcome in [`ReloadStats`].
fn wait_for_outcome(seq: u64, label: Option<&'static str>) -> Outcome {
    Outcome::Wait(Box::new(move |world| {
        let stats = world.get_resource::<ReloadStats>()?;
        let outcome = stats.outcome(seq)?;
        Some(match &outcome.result {
            Ok(summary) => Ok(
                json!({ "path": outcome.path.display().to_string(), "kind": label, "result": summary }),
            ),
            Err(error) => Err(
                RpcError::new(code::FAILED, format!("reload failed: {error}"))
                    .with_data(json!({ "path": outcome.path.display().to_string() })),
            ),
        })
    }))
}

fn reload_asset(world: &mut World, params: &Value) -> Outcome {
    let raw = match str_param(params, "path") {
        Ok(p) => PathBuf::from(p),
        Err(e) => return Outcome::err(e),
    };
    let path = match world.resource::<PathPolicy>().check_read(&raw) {
        Ok(p) => p,
        Err(e) => return Outcome::err(e),
    };
    let Some(kind) = classify(&path) else {
        return Outcome::err(RpcError::invalid_params(format!(
            "{} is not a reloadable file type",
            path.display()
        )));
    };
    let Some(mut stats) = world.get_resource_mut::<ReloadStats>() else {
        return Outcome::err(not_ready());
    };
    let seq = stats.allocate();
    world.send_event(ReloadRequested { path, kind, seq });
    wait_for_outcome(seq, Some(kind.label()))
}

fn reload_scene(world: &mut World, _: &Value) -> Outcome {
    if world.get_resource::<ReloadApplierRegistered>().is_none() {
        return Outcome::err(RpcError::new(
            code::UNSUPPORTED,
            "this application does not rebuild scenes",
        ));
    }
    let Some(mut stats) = world.get_resource_mut::<ReloadStats>() else {
        return Outcome::err(not_ready());
    };
    let seq = stats.allocate();
    world.send_event(ReloadSceneRequested { seq });
    wait_for_outcome(seq, None)
}

fn with_watcher(
    world: &World,
    params: &Value,
    f: impl FnOnce(&FileWatcher, PathBuf) -> Result<Value, RpcError>,
) -> Outcome {
    let raw = match str_param(params, "path") {
        Ok(p) => PathBuf::from(p),
        Err(e) => return Outcome::err(e),
    };
    let Some(watcher) = world.get_resource::<FileWatcher>() else {
        return Outcome::err(not_ready());
    };
    match world
        .resource::<PathPolicy>()
        .check_read(&raw)
        .and_then(|p| f(watcher, p))
    {
        Ok(v) => Outcome::ok(v),
        Err(e) => Outcome::err(e),
    }
}

fn watch_add(world: &mut World, params: &Value) -> Outcome {
    with_watcher(world, params, |w, path| {
        let dir = if path.is_dir() {
            path.clone()
        } else {
            path.parent().map(PathBuf::from).unwrap_or(path.clone())
        };
        let added = w.add(&dir, path.is_dir()).map_err(|e| {
            RpcError::new(code::FAILED, format!("cannot watch {}: {e}", dir.display()))
        })?;
        Ok(json!({ "path": dir.display().to_string(), "added": added }))
    })
}

fn watch_remove(world: &mut World, params: &Value) -> Outcome {
    with_watcher(world, params, |w, path| {
        Ok(json!({ "path": path.display().to_string(), "removed": w.remove(&path) }))
    })
}

fn watch_list(world: &mut World, _: &Value) -> Outcome {
    match world.get_resource::<FileWatcher>() {
        Some(w) => {
            let roots: Vec<Value> = w
                .roots()
                .into_iter()
                .map(|(p, r)| json!({ "path": p.display().to_string(), "recursive": r }))
                .collect();
            Outcome::ok(json!({ "roots": roots }))
        }
        None => Outcome::err(not_ready()),
    }
}
