//! The loading screen's stages, the manifest and a missing map.


use super::*;


#[test]
pub(super) fn the_screen_shows_stages_and_the_manifest_and_a_cancel_button() {
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
pub(super) fn the_manifest_names_the_map_that_really_loads_not_the_one_the_menu_chose() {
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
pub(super) fn a_missing_map_fails_and_the_error_is_never_overwritten() {
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
