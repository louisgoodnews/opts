//! Control protocol of loads that end with problems (broken tiles ...): `loading.continue`,
//! `loading.accept_problems` and the `accept_problems` parameter of the loading commands.

mod accept;
mod load_map;

use accept::loading_accept_problems;
pub(crate) use accept::{accept_problems_param, accepted_and_starting};
pub(crate) use load_map::load_map;

use bevy::prelude::*;
use opts_bevy::control::state::{ControlRegistry, Outcome};
use opts_cli::protocol::{code, RpcError};
use serde_json::{json, Value};

use crate::in_game::load_report::{AcceptLoadProblems, LoadReport};
use crate::in_game::types::SimulationLoadState;

/// Registers the methods.
pub fn register(registry: &mut ControlRegistry) {
    registry.register("loading.continue", loading_continue);
    registry.register("loading.accept_problems", loading_accept_problems);
}

/// `loading.continue {accept_all?}`: presses *Continue* on a loading screen that waits with
/// problems (broken tiles, a part of the vehicle that did not load). Without a waiting screen
/// it does nothing and says why. `accept_all: true` also lets the next load start by itself.
fn loading_continue(world: &mut World, params: &Value) -> Outcome {
    use crate::in_game::load_report::LoadOutcome;
    match params.get("accept_all") {
        None | Some(Value::Null) => {}
        Some(Value::Bool(on)) => world.insert_resource(AcceptLoadProblems(*on)),
        Some(_) => {
            return Outcome::err(RpcError::invalid_params("`accept_all` must be a boolean"));
        }
    }
    let started = world
        .get_resource::<SimulationLoadState>()
        .is_some_and(|s| s.hud_spawned);
    let Some(mut report) = world.get_resource_mut::<LoadReport>() else {
        return Outcome::err(RpcError::new(code::NOT_READY, "no loading screen"));
    };
    let problems: Vec<Value> = report
        .problems
        .iter()
        .map(|p| json!({ "key": p.key, "args": p.args, "fatal": p.fatal, "detail": p.detail }))
        .collect();
    let outcome = format!("{:?}", report.outcome);
    let continued =
        report.outcome == LoadOutcome::ReadyWithProblems && !report.continue_requested && !started;
    if continued {
        report.continue_requested = true;
    }
    if report.outcome == LoadOutcome::Failed {
        return Outcome::err(RpcError::new(
            code::FAILED,
            format!("the load failed, there is nothing to continue: {outcome}"),
        ));
    }
    Outcome::ok(json!({ "continued": continued, "outcome": outcome, "problems": problems }))
}

#[cfg(test)]
mod tests;
