use super::*;
use crate::in_game::load_report::{LoadOutcome, LoadReport};

fn world_in(state: AppState) -> World {
    let mut w = World::new();
    w.insert_resource(State::new(state));
    w.insert_resource(NextState::<AppState>::default());
    w.init_resource::<LoadReport>();
    w.init_resource::<SimulationLoadState>();
    w.init_resource::<WorldStats>();
    w
}

/// Runs the waiter through the menu and into a new session, with `outcome` as the load result.
fn answer(outcome: LoadOutcome) -> Option<Result<Value, RpcError>> {
    let mut w = world_in(AppState::InGame);
    let mut waiter = restart_waiter(&mut w);
    assert!(waiter(&mut w).is_none(), "still in the old session");
    w.insert_resource(State::new(AppState::MainMenu));
    assert!(
        waiter(&mut w).is_none(),
        "left the session, new one requested"
    );
    w.insert_resource(State::new(AppState::InGame));
    w.resource_mut::<LoadReport>().outcome = outcome;
    waiter(&mut w)
}

/// MI-0036: a failed load answers `FAILED` at once.
#[test]
fn a_failed_load_answers_with_an_error() {
    let r = answer(LoadOutcome::Failed).expect("answers");
    assert_eq!(r.expect_err("error").code, code::FAILED);
}

/// MI-0036: a load with problems answers while the loading screen waits for Continue.
#[test]
fn a_load_with_problems_does_not_wait_for_continue() {
    let r = answer(LoadOutcome::ReadyWithProblems)
        .expect("answers")
        .expect("ok");
    assert_eq!(r["waiting_for_continue"], json!(true));
}

#[test]
fn a_load_in_progress_keeps_waiting() {
    assert!(answer(LoadOutcome::Loading).is_none());
}
