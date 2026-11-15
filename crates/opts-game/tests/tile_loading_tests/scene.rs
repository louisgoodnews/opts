//! The scene.tiles control method: mode, radius, resident tiles and readiness.


use super::*;
use opts_bevy::control::channel::dispatch;
use opts_bevy::control::state::{ControlRegistry, Outcome};
use serde_json::{json, Value};


/// `scene.tiles` through the dispatcher, with only that method registered.
pub(super) fn scene_tiles(app: &mut App) -> Value {
    if app.world().get_resource::<ControlRegistry>().is_none() {
        let mut registry = ControlRegistry::default();
        opts_game::control_tiles::register(&mut registry);
        app.insert_resource(registry);
    }
    match dispatch(app.world_mut(), "scene.tiles", &json!({})) {
        Outcome::Done(r) => r.expect("scene.tiles answers"),
        Outcome::Wait(_) => panic!("scene.tiles does not wait"),
    }
}

#[test]
pub(super) fn scene_tiles_reports_the_mode_the_radius_and_the_resident_tiles() {
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
pub(super) fn scene_tiles_says_not_ready_outside_a_drive() {
    let mut app = App::new();
    let mut registry = ControlRegistry::default();
    opts_game::control_tiles::register(&mut registry);
    app.insert_resource(registry);
    match dispatch(app.world_mut(), "scene.tiles", &json!({})) {
        Outcome::Done(Err(e)) => assert_eq!(e.code, opts_cli::protocol::code::NOT_READY),
        _ => panic!("expected NOT_READY"),
    }
}
