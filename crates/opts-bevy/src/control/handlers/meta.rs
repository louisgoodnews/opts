//! `ping`, `version`, `help`, `capabilities`, `shutdown`.

use bevy::app::AppExit;
use bevy::ecs::world::World;
use opts_cli::commands::{self, COMMANDS};
use opts_cli::protocol::{code, RpcError, PROTOCOL_VERSION};
use serde_json::{json, Value};

use crate::control::params::{opt_bool, opt_str};
use crate::control::state::{ControlRegistry, ControlState, Outcome};

/// Registers the meta methods.
pub fn register(r: &mut ControlRegistry) {
    r.register("ping", ping);
    r.register("version", version);
    r.register("help", help);
    r.register("capabilities", capabilities);
    r.register("shutdown", shutdown);
}

fn ping(world: &mut World, _: &Value) -> Outcome {
    let up = world.resource::<ControlState>().started.elapsed().as_secs();
    Outcome::ok(json!({ "pong": true, "uptime_secs": up }))
}

fn version(_: &mut World, _: &Value) -> Outcome {
    Outcome::ok(json!({
        "version": env!("CARGO_PKG_VERSION"),
        "git_hash": option_env!("OPTS_GIT_HASH").unwrap_or("unknown"),
        "build_date": option_env!("OPTS_BUILD_DATE").unwrap_or("unknown"),
        "protocol_version": PROTOCOL_VERSION,
    }))
}

fn help(world: &mut World, params: &Value) -> Outcome {
    let name = match opt_str(params, "command") {
        Ok(n) => n,
        Err(e) => return Outcome::err(e),
    };
    let registry = world.resource::<ControlRegistry>();
    let Some(name) = name else {
        let all: Vec<Value> = COMMANDS
            .iter()
            .map(|c| json!({ "method": c.method, "summary": c.summary }))
            .collect();
        return Outcome::ok(json!({ "commands": all }));
    };
    match commands::find(name) {
        Some(c) => Outcome::ok(json!({
            "command": c.method,
            "group": c.group,
            "summary": c.summary,
            "params": c.params,
            "implemented": registry.contains(c.method),
        })),
        None => Outcome::err(RpcError::new(
            code::NOT_FOUND,
            format!("no command `{name}`"),
        )),
    }
}

fn capabilities(world: &mut World, _: &Value) -> Outcome {
    let registry = world.resource::<ControlRegistry>();
    let mut groups = serde_json::Map::new();
    for c in COMMANDS {
        let entry = groups
            .entry(c.group.to_string())
            .or_insert_with(|| json!([]));
        if let Some(list) = entry.as_array_mut() {
            list.push(json!({ "method": c.method, "since": "1.0", "implemented": registry.contains(c.method) }));
        }
    }
    Outcome::ok(json!({ "protocol_version": PROTOCOL_VERSION, "groups": groups }))
}

fn shutdown(world: &mut World, params: &Value) -> Outcome {
    let force = match opt_bool(params, "force") {
        Ok(f) => f.unwrap_or(false),
        Err(e) => return Outcome::err(e),
    };
    tracing::info!(target: "opts_bevy::control", force, "shutdown requested over the control protocol");
    world.send_event(AppExit::Success);
    Outcome::ok(json!({ "ok": true }))
}
