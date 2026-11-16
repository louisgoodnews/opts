//! Integration tests for accepted load problems: drive start without continue and the report entries.
use super::*;

#[test]
fn accepted_problems_start_the_drive_without_continue_and_are_logged_in_the_report() {
    use opts_game::in_game::load_report::AcceptLoadProblems;
    let root = broken_tile_root("tile-accept");
    let mut app = loading_app("Mini", "MiniBus", args_for(&root));
    app.insert_resource(AcceptLoadProblems(true));
    run(&mut app, 60);
    assert_eq!(outcome(&app), LoadOutcome::ReadyWithProblems);
    assert_eq!(screen_count(&mut app), 0, "no Continue needed");
    assert!(app.world().resource::<SimulationLoadState>().hud_spawned);
    assert!(
        app.world()
            .resource::<LoadReport>()
            .problems
            .iter()
            .any(|p| p.key == "load-problem-tiles"),
        "the problem stays in the report"
    );
    assert!(
        !app.world().resource::<AcceptLoadProblems>().0,
        "the protocol's acceptance covers one load"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn the_command_line_flag_accepts_problems_for_every_load() {
    let root = broken_tile_root("tile-flag");
    let mut args = args_for(&root);
    args.accept_load_problems = true;
    let mut app = loading_app("Mini", "MiniBus", args);
    run(&mut app, 60);
    assert_eq!(screen_count(&mut app), 0);
    assert!(app.world().resource::<SimulationLoadState>().hud_spawned);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn the_protocols_always_switch_accepts_problems_and_stays_on() {
    use opts_game::in_game::load_report::AcceptLoadProblemsAlways;
    let root = broken_tile_root("tile-always");
    let mut app = loading_app("Mini", "MiniBus", args_for(&root));
    app.insert_resource(AcceptLoadProblemsAlways(true));
    run(&mut app, 60);
    assert_eq!(outcome(&app), LoadOutcome::ReadyWithProblems);
    assert_eq!(screen_count(&mut app), 0, "no Continue needed");
    assert!(app.world().resource::<SimulationLoadState>().hud_spawned);
    assert!(
        app.world().resource::<AcceptLoadProblemsAlways>().0,
        "unlike the one-load acceptance it stays on for the next load"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_vehicle_that_cannot_load_is_a_problem_and_the_text_stays() {
    let root = fresh_root("vehicle");
    write_map(&root, &MiniMap::simple("Mini"));
    let mut app = loading_app("Mini", "MiniBus", args_for(&root));
    run(&mut app, 40);
    assert_eq!(outcome(&app), LoadOutcome::ReadyWithProblems);
    let keys: Vec<_> = app
        .world()
        .resource::<LoadReport>()
        .problems
        .iter()
        .map(|p| p.key)
        .collect();
    assert!(keys.contains(&"load-problem-no-model"), "{keys:?}");
    assert!(keys.contains(&"load-problem-vehicle"), "{keys:?}");
    run(&mut app, 30);
    let t = texts(&mut app);
    assert!(
        t.iter().any(|x| x == "Couldn't load the vehicle MiniBus"),
        "{t:?}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn failed_assets_are_counted_into_the_report() {
    let root = fresh_root("assets");
    let mut map = MiniMap::simple("Mini");
    map.dangling_objects = 3;
    write_map(&root, &map);
    let mut app = loading_app("Mini", "MiniBus", args_for(&root));
    run(&mut app, 40);
    let report = app.world().resource::<LoadReport>();
    assert!(
        report
            .problems
            .iter()
            .any(|p| p.key == "load-problem-files" || p.key == "load-problem-objects"),
        "{:?}",
        report.problems
    );
    assert!(report.minor_count() > 0, "failed assets are minor problems");
    let _ = std::fs::remove_dir_all(&root);
}
