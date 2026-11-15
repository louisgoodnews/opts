use super::*;

#[test]
fn scene_tiles_reports_the_mode_the_radius_and_the_resident_tiles() {
    let (mut app, root) = drive("tl-proto", &[("gameplay.tile_radius", "2")]);
    let r = scene_tiles(&mut app);
    assert_eq!(r["mode"], "radius");
    assert_eq!(r["radius"], 2);
    assert_eq!(r["automatic"], false);
    assert_eq!(r["setting_radius"], 2);
    assert_eq!(r["load_all"], false);
    assert_eq!(r["map_radius"], 4);
    assert_eq!(r["listed"], 5);
    assert_eq!(r["listed_missing"], 0);
    assert_eq!(r["start_tile"], json!([0, 0]));
    assert_eq!(r["resident_count"], 3);
    assert_eq!(r["resident"], json!([[0, 0], [1, 0], [2, 0]]));
    assert_eq!(r["queued"], 0);

    app.world_mut()
        .resource_mut::<GameSettings>()
        .set_raw("gameplay.load_all_tiles", "true");
    run(&mut app, 30);
    let r = scene_tiles(&mut app);
    assert_eq!(r["mode"], "all");
    assert_eq!(r["radius"], Value::Null);
    assert_eq!(r["load_all"], true);
    assert_eq!(r["resident_count"], 5);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn scene_tiles_says_not_ready_outside_a_drive() {
    let mut app = App::new();
    let mut registry = ControlRegistry::default();
    opts_game::control_tiles::register(&mut registry);
    app.insert_resource(registry);
    match dispatch(app.world_mut(), "scene.tiles", &json!({})) {
        Outcome::Done(Err(e)) => assert_eq!(e.code, opts_cli::protocol::code::NOT_READY),
        _ => panic!("expected NOT_READY"),
    }
}

#[test]
fn scene_tiles_reports_the_tile_in_progress_and_the_budget() {
    let (mut app, root) = drive("tl-report", &[("gameplay.tile_radius", "1")]);
    let r = scene_tiles(&mut app);
    assert_eq!(r["building"], Value::Null);
    assert_eq!(r["stream_budget_ms"], 8);
    set_budget(&mut app, std::time::Duration::from_nanos(1));
    move_camera_to(&mut app, (4, 0));
    for _ in 0..30 {
        app.update();
        if building(&app).is_some() {
            break;
        }
    }
    let r = scene_tiles(&mut app);
    assert!(r["building"].is_array(), "{r}");
    assert_eq!(r["stream_budget_ms"], 0, "below a millisecond");
    let _ = std::fs::remove_dir_all(&root);
}
