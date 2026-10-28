//! Unit tests for `super` (`control_loading.rs`).

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
    w.resource_mut::<LoadReport>()
        .problems
        .push(crate::in_game::load_report::LoadProblem::new(
            false,
            "load-problem-tiles",
            "Tram",
            "tile 3 broken",
        ));
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
fn accept_problems_always_stays_until_switched_off() {
    let mut w = world_in(AppState::MainMenu);
    let Outcome::Done(Ok(v)) = loading_accept_problems(&mut w, &json!({"always": true})) else {
        panic!("answers at once")
    };
    assert_eq!(v["always"], json!(true));
    // Reading leaves it on; `false` switches it off; a non-boolean is refused.
    let Outcome::Done(Ok(v)) = loading_accept_problems(&mut w, &json!({})) else {
        panic!("answers at once")
    };
    assert_eq!(v["always"], json!(true));
    let Outcome::Done(Ok(v)) = loading_accept_problems(&mut w, &json!({"always": false})) else {
        panic!("answers at once")
    };
    assert_eq!(v["always"], json!(false));
    assert!(matches!(
        loading_accept_problems(&mut w, &json!({"always": 1})),
        Outcome::Done(Err(_))
    ));
    // `scene.load_map {accept_problems: "always"}` sets the same switch.
    accept_problems_param(&mut w, &json!({"accept_problems": "always"})).expect("accepted");
    assert!(
        w.resource::<crate::in_game::load_report::AcceptLoadProblemsAlways>()
            .0
    );
    assert!(accept_problems_param(&mut w, &json!({"accept_problems": "often"})).is_err());
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
