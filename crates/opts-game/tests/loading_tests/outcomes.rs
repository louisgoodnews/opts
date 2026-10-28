//! Loading screen tests for retry, overlay, menu camera, CI rules and minor problems.

use super::*;

#[test]
fn retry_starts_the_load_again_and_clears_the_report() {
    let root = fresh_root("retry");
    std::fs::create_dir_all(root.join("maps")).expect("maps");
    let mut app = loading_app("Ghost Town", "MiniBus", args_for(&root));
    run(&mut app, 12);
    assert_eq!(outcome(&app), LoadOutcome::Failed);
    // The map appears; the retry finds it.
    write_map(&root, &MiniMap::simple("Ghost Town"));
    let retry = find_entity(app.world_mut(), "loading.retry").expect("retry");
    app.world_mut().write_message(Activated(retry));
    run(&mut app, 40);
    assert_ne!(
        outcome(&app),
        LoadOutcome::Failed,
        "{:?}",
        app.world().resource::<LoadReport>().problems
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn the_developer_overlay_is_hidden_while_the_loading_screen_is_up() {
    let root = fresh_root("overlay");
    write_map(&root, &MiniMap::simple("Mini"));
    let mut app = loading_app("Mini", "MiniBus", args_for(&root));
    run(&mut app, 40);
    assert_eq!(screen_count(&mut app), 1, "problems keep the screen up");
    let mut q = app.world_mut().query::<(
        &Visibility,
        &opts_game::in_game::omsi_world::DebugOverlayText,
    )>();
    let all: Vec<_> = q.iter(app.world()).map(|(v, _)| *v).collect();
    assert!(!all.is_empty(), "the overlay exists");
    assert!(all.iter().all(|v| *v == Visibility::Hidden), "{all:?}");
    let _ = std::fs::remove_dir_all(&root);
}

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
