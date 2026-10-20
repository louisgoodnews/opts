//! Control protocol of loads that end with problems (broken tiles ...): `loading.continue` and the
//! `accept_problems` parameter of the loading commands.

use bevy::prelude::*;
use opts_bevy::control::state::{ControlRegistry, Outcome};
use opts_cli::protocol::{code, RpcError};
use serde_json::{json, Value};

use crate::in_game::types::SimulationLoadState;

/// Registers the methods.
pub fn register(registry: &mut ControlRegistry) {
    registry.register("loading.continue", loading_continue);
}

/// `accept_problems: true` on a load: the drive starts without waiting for *Continue* if the load
/// ends with problems (one load only).
pub(crate) fn accept_problems_param(world: &mut World, params: &Value) -> Result<(), RpcError> {
    match params.get("accept_problems") {
        None | Some(Value::Null) => Ok(()),
        Some(Value::Bool(on)) => {
            world.insert_resource(crate::in_game::load_report::AcceptLoadProblems(*on));
            Ok(())
        }
        Some(_) => Err(RpcError::invalid_params(
            "`accept_problems` must be a boolean",
        )),
    }
}

/// `loading.continue {accept_all?}`: presses *Continue* on a loading screen that waits with
/// problems (broken tiles, a part of the vehicle that did not load). Without a waiting screen
/// it does nothing and says why. `accept_all: true` also lets the next load start by itself.
fn loading_continue(world: &mut World, params: &Value) -> Outcome {
    use crate::in_game::load_report::{AcceptLoadProblems, LoadOutcome, LoadReport};
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
mod tests {
    use super::*;
    use crate::app_state::AppState;
    use crate::in_game::load_report::{LoadOutcome, LoadReport};
    use crate::in_game::omsi_world::WorldStats;

    fn world_in(state: AppState) -> World {
        let mut w = World::new();
        w.insert_resource(State::new(state));
        w.init_resource::<LoadReport>();
        w.init_resource::<SimulationLoadState>();
        w.init_resource::<WorldStats>();
        w
    }

    fn continue_with(outcome: LoadOutcome, params: Value) -> (Outcome, World) {
        let mut w = world_in(AppState::InGame);
        w.resource_mut::<LoadReport>().outcome = outcome;
        w.resource_mut::<LoadReport>().problems.push(
            crate::in_game::load_report::LoadProblem::new(
                false,
                "load-problem-tiles",
                "Tram",
                "tile 3 broken",
            ),
        );
        (loading_continue(&mut w, &params), w)
    }

    /// `loading.continue` presses Continue on a load that waits with problems and lists them.
    #[test]
    fn loading_continue_passes_a_waiting_load_and_lists_the_problems() {
        let (out, w) = continue_with(LoadOutcome::ReadyWithProblems, json!({}));
        let Outcome::Done(Ok(v)) = out else {
            panic!("answers at once")
        };
        assert_eq!(v["continued"], json!(true));
        assert_eq!(v["problems"][0]["key"], json!("load-problem-tiles"));
        assert!(w.resource::<LoadReport>().continue_requested);
    }

    #[test]
    fn loading_continue_does_nothing_while_loading_and_refuses_a_failed_load() {
        let (out, w) = continue_with(LoadOutcome::Loading, json!({}));
        let Outcome::Done(Ok(v)) = out else {
            panic!("answers at once")
        };
        assert_eq!(v["continued"], json!(false));
        assert!(!w.resource::<LoadReport>().continue_requested);
        let (out, _) = continue_with(LoadOutcome::Failed, json!({}));
        let Outcome::Done(Err(e)) = out else {
            panic!("an error")
        };
        assert_eq!(e.code, code::FAILED);
    }

    #[test]
    fn accept_all_is_remembered_for_the_next_load() {
        let (_, w) = continue_with(LoadOutcome::Loading, json!({"accept_all": true}));
        assert!(
            w.resource::<crate::in_game::load_report::AcceptLoadProblems>()
                .0
        );
        let (out, _) = continue_with(LoadOutcome::Loading, json!({"accept_all": "yes"}));
        assert!(matches!(out, Outcome::Done(Err(_))));
    }
}
