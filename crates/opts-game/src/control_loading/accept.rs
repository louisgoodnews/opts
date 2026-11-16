//! The switches of loads that end with problems: `loading.accept_problems`, and the
//! `accept_problems` parameter of the loading commands.

use bevy::prelude::*;
use opts_bevy::control::state::Outcome;
use opts_cli::protocol::RpcError;
use serde_json::{json, Value};

use crate::in_game::load_report::{AcceptLoadProblems, AcceptLoadProblemsAlways, LoadReport};
use crate::in_game::types::SimulationLoadState;

/// `loading.accept_problems {always?}`: with `always` set, every later load that ends with
/// problems starts its drive without waiting for *Continue* (wizard loads too) until it is set to
/// `false`; answers the switches in force either way.
pub(super) fn loading_accept_problems(world: &mut World, params: &Value) -> Outcome {
    match params.get("always") {
        None | Some(Value::Null) => {}
        Some(Value::Bool(on)) => world.insert_resource(AcceptLoadProblemsAlways(*on)),
        Some(_) => return Outcome::err(RpcError::invalid_params("`always` must be a boolean")),
    }
    let always = world
        .get_resource::<AcceptLoadProblemsAlways>()
        .is_some_and(|a| a.0);
    let next_load = world
        .get_resource::<AcceptLoadProblems>()
        .is_some_and(|a| a.0);
    let flag = world
        .get_resource::<crate::AppArgs>()
        .is_some_and(|a| a.0.accept_load_problems);
    Outcome::ok(json!({ "always": always, "next_load": next_load, "command_line": flag }))
}

/// Whether a load that ended with problems is accepted (by *Continue*, the protocol or the
/// command line) and its drive has not begun yet: the loading commands answer once it has.
pub(crate) fn accepted_and_starting(world: &World, report: &LoadReport) -> bool {
    let accepted = report.continue_requested
        || world
            .get_resource::<AcceptLoadProblems>()
            .is_some_and(|a| a.0)
        || world
            .get_resource::<AcceptLoadProblemsAlways>()
            .is_some_and(|a| a.0)
        || world
            .get_resource::<crate::AppArgs>()
            .is_some_and(|a| a.0.accept_load_problems);
    let started = world
        .get_resource::<SimulationLoadState>()
        .is_some_and(|s| s.hud_spawned);
    accepted && !started
}

/// `accept_problems: true` on a load: the drive starts without waiting for *Continue* if the load
/// ends with problems (one load only); `"always"` does so for this and every later load.
pub(crate) fn accept_problems_param(world: &mut World, params: &Value) -> Result<(), RpcError> {
    match params.get("accept_problems") {
        None | Some(Value::Null) => Ok(()),
        Some(Value::Bool(on)) => {
            world.insert_resource(AcceptLoadProblems(*on));
            Ok(())
        }
        Some(Value::String(s)) if s == "always" => {
            world.insert_resource(AcceptLoadProblemsAlways(true));
            Ok(())
        }
        Some(_) => Err(RpcError::invalid_params(
            "`accept_problems` must be a boolean or \"always\"",
        )),
    }
}
