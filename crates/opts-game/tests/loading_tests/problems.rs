//! Missing, failed and minor problems on the loading screen.

use super::*;

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
fn only_minor_problems_continue_by_themselves_and_are_announced_once() {
    use opts_game::in_game::load_report::LoadProblem;
    let mut app = loading_app(
        "Mini",
        "MiniBus",
        CliArgs {
            headless: true,
            ..CliArgs::default()
        },
    );
    {
        let mut s = app.world_mut().resource_mut::<SimulationLoadState>();
        s.stage = LoadStage::Ready;
        s.is_complete = true;
        let mut r = app.world_mut().resource_mut::<LoadReport>();
        r.reset();
        r.add(LoadProblem::counted(
            "load-problem-files",
            2,
            0,
            "N2_White.tga",
        ));
        r.finish();
    }
    assert_eq!(
        outcome(&app),
        LoadOutcome::Ready,
        "minor problems do not block"
    );
    run(&mut app, 6);
    assert_eq!(screen_count(&mut app), 0, "the screen continues by itself");
    run(&mut app, 4);
    let t = texts(&mut app);
    assert_eq!(
        t.iter().filter(|x| x.starts_with("Loaded with")).count(),
        1,
        "one toast: {t:?}"
    );
    assert!(
        t.iter().any(|x| x == "Loaded with 1 minor problem"),
        "{t:?}"
    );
    assert_eq!(
        app.world().resource::<LoadReport>().minor_count(),
        1,
        "kept in the report"
    );
    // Details opens the list.
    let action = app
        .world_mut()
        .query::<(Entity, &opts_gui::components::toast::ToastActionButton)>()
        .iter(app.world())
        .next()
        .map(|(e, _)| e)
        .expect("the toast has a Details action");
    app.world_mut().write_message(Activated(action));
    run(&mut app, 3);
    assert!(has(&mut app, "loading.minor-dialog"));
    let t = texts(&mut app);
    assert!(
        t.iter().any(|x| x.contains("files could not be read")),
        "{t:?}"
    );
    run(&mut app, 30);
    let n = texts(&mut app)
        .iter()
        .filter(|x| x.starts_with("Loaded with"))
        .count();
    assert!(n <= 1, "announced once");
}
