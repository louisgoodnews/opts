//! The menu camera and the CI rules of the loading screen in each outcome.

use super::*;

#[test]
fn the_menu_camera_keeps_drawing_the_loading_screen_until_a_scene_camera_exists() {
    // Without a camera the window stays black: the failed outcome never gets a scene camera.
    let root = fresh_root("camera");
    std::fs::create_dir_all(root.join("maps")).expect("maps");
    let mut app = loading_app("Ghost Town", "MiniBus", args_for(&root));
    let cam = app
        .world_mut()
        .spawn((Camera2d, opts_game::MenuCamera))
        .id();
    run(&mut app, 20);
    assert_eq!(outcome(&app), LoadOutcome::Failed);
    let active = app.world().get::<Camera>(cam).map(|c| c.is_active);
    assert_eq!(
        active,
        Some(true),
        "the failed screen needs the menu camera"
    );
    // A scene camera appears: now the menu camera steps aside.
    app.world_mut().spawn((
        Camera3d::default(),
        opts_bevy::MainCamera,
        Camera::default(),
    ));
    run(&mut app, 2);
    let active = app.world().get::<Camera>(cam).map(|c| c.is_active);
    assert_eq!(active, Some(false));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn the_loading_screen_meets_the_ci_rules_in_every_outcome() {
    use opts_gui::testing::rules;
    let root = fresh_root("rules");
    write_map(&root, &MiniMap::simple("Mini"));
    for failing in [false, true] {
        let map = if failing { "Ghost Town" } else { "Mini" };
        let mut app = loading_app(map, "MiniBus", args_for(&root));
        run(&mut app, 40);
        let want = if failing {
            LoadOutcome::Failed
        } else {
            LoadOutcome::ReadyWithProblems
        };
        assert_eq!(outcome(&app), want);
        for (name, found) in [
            ("targets", rules::small_targets(app.world_mut())),
            ("text", rules::small_text(app.world_mut())),
            ("disabled", rules::disabled_without_reason(app.world_mut())),
        ] {
            assert!(found.is_empty(), "{map} {name}: {}", found.join("\n"));
        }
        let t = rules::keyboard_traversal(&mut app);
        assert!(t.missed.is_empty(), "{map}: {:?}", t.missed);
    }
    let _ = std::fs::remove_dir_all(&root);
}
