//! The loading screen on self-authored miniature installations: stages, the manifest of the real
//! session, seeded failures that must stay visible, and the three outcomes.

mod common;

use bevy::prelude::*;
use opts_game::{
    app_state::AppState,
    in_game::{
        load_report::{LoadOutcome, LoadReport},
        CancelLoadingBtn, InGameEntity, InGameHudRoot, LoadStage, SimulationLoadState,
    },
    CliArgs,
};
use opts_gui::interaction::Activated;
use opts_gui::ui_tree::find_entity;

use common::loading::{args_for, has, loading_app, outcome, run, screen_count, texts};
use common::{fresh_root, write_map, MiniMap};

#[test]
fn the_screen_shows_stages_and_the_manifest_and_a_cancel_button() {
    let root = fresh_root("screen");
    write_map(&root, &MiniMap::simple("Mini"));
    let mut app = loading_app("Mini", "MiniBus", args_for(&root));
    run(&mut app, 2);
    assert_eq!(screen_count(&mut app), 1);
    let t = texts(&mut app);
    assert!(t.iter().any(|x| x == "Preparing your drive"), "{t:?}");
    assert!(t.iter().any(|x| x == "CONTROLS"));
    assert!(
        !t.iter()
            .any(|x| x.contains("Start Driving") || x.contains("TRANSIT")),
        "no developer wording, no unusable button: {t:?}"
    );
    assert!(has(&mut app, "loading.cancel"));
    assert!(!has(&mut app, "loading.start"));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn the_manifest_names_the_map_that_really_loads_not_the_one_the_menu_chose() {
    // F17: the menu says "Berlin-Spandau"; the command line points at "Mini". The card must
    // name what loads.
    let root = fresh_root("f17");
    let dir = write_map(&root, &MiniMap::simple("Mini"));
    let mut args = args_for(&root);
    args.map_path = Some(dir);
    let mut app = loading_app("Berlin-Spandau", "MiniBus", args);
    run(&mut app, 6);
    let t = texts(&mut app);
    assert!(
        t.iter().any(|x| x == "Mini"),
        "the loading map is named: {t:?}"
    );
    assert!(
        !t.iter().any(|x| x == "Berlin-Spandau"),
        "the menu's map must not be on the card: {t:?}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_missing_map_fails_and_the_error_is_never_overwritten() {
    let root = fresh_root("nomap");
    std::fs::create_dir_all(root.join("maps")).expect("maps");
    std::fs::create_dir_all(root.join("Vehicles")).expect("vehicles");
    let mut app = loading_app("Ghost Town", "MiniBus", args_for(&root));
    run(&mut app, 12);
    assert_eq!(outcome(&app), LoadOutcome::Failed);
    assert_eq!(
        app.world().resource::<SimulationLoadState>().stage,
        LoadStage::Failed
    );
    let before = texts(&mut app);
    assert!(
        before
            .iter()
            .any(|x| x == "Couldn't load the map Ghost Town"),
        "{before:?}"
    );
    // Many more frames: the sentence stays, the screen stays, nothing continues by itself.
    run(&mut app, 60);
    let after = texts(&mut app);
    assert!(after
        .iter()
        .any(|x| x == "Couldn't load the map Ghost Town"));
    assert_eq!(screen_count(&mut app), 1);
    assert!(!app.world().resource::<SimulationLoadState>().hud_spawned);
    for id in ["loading.retry", "loading.choose-another", "loading.back"] {
        assert!(has(&mut app, id), "{id} missing");
    }
    assert!(!has(&mut app, "loading.continue"));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_missing_map_tile_is_reported_and_the_screen_waits_for_continue() {
    let root = fresh_root("tile");
    let map = MiniMap {
        name: "Mini",
        listed: vec![(0, 0), (1, 0)],
        written: vec![(0, 0)],
        dangling_objects: 0,
    };
    write_map(&root, &map);
    let mut app = loading_app("Mini", "MiniBus", args_for(&root));
    run(&mut app, 40);
    assert_eq!(outcome(&app), LoadOutcome::ReadyWithProblems);
    let report = app.world().resource::<LoadReport>().clone();
    assert!(
        report
            .problems
            .iter()
            .any(|p| p.key == "load-problem-tiles"),
        "{:?}",
        report.problems
    );
    let t = texts(&mut app);
    assert!(
        t.iter()
            .any(|x| x.starts_with("Ready with ") && x.ends_with("problems")
                || x.starts_with("Ready with 1 problem")),
        "{t:?}"
    );
    assert!(
        t.iter().any(|x| x.contains("map tiles are missing")),
        "{t:?}"
    );
    run(&mut app, 60);
    assert_eq!(screen_count(&mut app), 1, "problems stop the auto-continue");
    // Continue releases it.
    let cont = find_entity(app.world_mut(), "loading.continue").expect("Continue");
    app.world_mut().write_message(Activated(cont));
    run(&mut app, 6);
    assert_eq!(screen_count(&mut app), 0);
    assert!(app.world().resource::<SimulationLoadState>().hud_spawned);
    let _ = std::fs::remove_dir_all(&root);
}

fn broken_tile_root(tag: &str) -> std::path::PathBuf {
    let root = fresh_root(tag);
    write_map(
        &root,
        &MiniMap {
            name: "Mini",
            listed: vec![(0, 0), (1, 0)],
            written: vec![(0, 0)],
            dangling_objects: 0,
        },
    );
    root
}

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

#[test]
fn a_clean_load_continues_by_itself_without_a_button() {
    let mut app = loading_app(
        "Mini",
        "MiniBus",
        CliArgs {
            headless: true,
            ..CliArgs::default()
        },
    );
    // Skip the loader: the world is "done" and nothing went wrong.
    {
        let mut s = app.world_mut().resource_mut::<SimulationLoadState>();
        s.stage = LoadStage::Ready;
        s.is_complete = true;
    }
    {
        let mut r = app.world_mut().resource_mut::<LoadReport>();
        r.reset();
        r.finish();
    }
    assert_eq!(outcome(&app), LoadOutcome::Ready);
    run(&mut app, 6);
    assert_eq!(screen_count(&mut app), 0, "the screen continues by itself");
    assert!(app.world().resource::<SimulationLoadState>().hud_spawned);
    let hud = app
        .world_mut()
        .query_filtered::<Entity, With<InGameHudRoot>>()
        .iter(app.world())
        .count();
    assert_eq!(hud, 1);
}

#[test]
fn cancel_asks_first_and_then_returns_to_plan_a_drive() {
    let root = fresh_root("cancel");
    write_map(&root, &MiniMap::simple("Mini"));
    let mut app = loading_app("Mini", "MiniBus", args_for(&root));
    run(&mut app, 2);
    let cancel = app
        .world_mut()
        .query_filtered::<Entity, With<CancelLoadingBtn>>()
        .iter(app.world())
        .next()
        .expect("cancel button");
    app.world_mut().write_message(Activated(cancel));
    run(&mut app, 3);
    assert!(
        has(&mut app, "loading.cancel-dialog"),
        "the dialog asks first"
    );
    assert_eq!(
        *app.world().resource::<State<AppState>>().get(),
        AppState::InGame
    );
    let keep = find_entity(app.world_mut(), "loading.cancel-keep").expect("keep");
    app.world_mut().write_message(Activated(keep));
    run(&mut app, 3);
    assert!(!has(&mut app, "loading.cancel-dialog"));
    app.world_mut().write_message(Activated(cancel));
    run(&mut app, 3);
    let yes = find_entity(app.world_mut(), "loading.cancel-yes").expect("yes");
    app.world_mut().write_message(Activated(yes));
    run(&mut app, 4);
    assert_eq!(
        *app.world().resource::<State<AppState>>().get(),
        AppState::VehicleSelection
    );
    let left = app
        .world_mut()
        .query_filtered::<Entity, With<InGameEntity>>()
        .iter(app.world())
        .count();
    assert_eq!(left, 0, "the session is torn down");
    let _ = std::fs::remove_dir_all(&root);
}

#[path = "loading_tests/outcomes.rs"]
mod outcomes;
