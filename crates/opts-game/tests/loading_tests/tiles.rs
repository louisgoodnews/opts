//! A missing map tile is reported and the screen waits for Continue.


use super::*;


#[test]
pub(super) fn a_missing_map_tile_is_reported_and_the_screen_waits_for_continue() {
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

pub(super) fn broken_tile_root(tag: &str) -> std::path::PathBuf {
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
