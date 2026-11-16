//! Session load: refused for damaged, newer or missing content, and a good save starts a drive.

use super::*;

#[test]
fn session_load_refuses_damaged_newer_and_missing_content_and_starts_a_good_save() {
    let root = scratch("save-load-root");
    std::fs::create_dir_all(root.join("maps/Testmap")).expect("map");
    std::fs::create_dir_all(root.join("Vehicles")).expect("vehicles");
    std::fs::write(root.join("maps/Testmap/global.cfg"), "x").expect("cfg");
    std::env::set_var("OMSI_PATH", &root);
    let mut app = save_app("save-load");
    app.world_mut().resource_mut::<NewGameSetup>().selected_map = "Testmap".into();
    call(&mut app, "session.save", json!({"name": "Gut"})).expect("save");

    let dir = app
        .world()
        .resource::<SaveStorage>()
        .0
        .as_ref()
        .expect("store")
        .dir()
        .to_path_buf();
    std::fs::write(
        dir.join("kaputt.save.json"),
        "{\"magic\": \"OPTS-SAVE\", \"for",
    )
    .expect("bad");
    std::fs::write(
        dir.join("neu.save.json"),
        json!({"magic": "OPTS-SAVE", "format_version": 9}).to_string(),
    )
    .expect("newer");
    let e = call(&mut app, "session.load", json!({"name": "kaputt"})).expect_err("corrupt");
    assert_eq!(e.data.as_ref().expect("data")["kind"], "corrupt");
    let e = call(&mut app, "session.load", json!({"name": "neu"})).expect_err("newer");
    assert_eq!(e.data.as_ref().expect("data")["kind"], "newer_format");
    let e = call(&mut app, "session.load", json!({"name": "gibt-es-nicht"})).expect_err("unknown");
    assert_eq!(e.code, code::NOT_FOUND);

    // The same save with the map gone: refused with the issue.
    std::fs::remove_dir_all(root.join("maps/Testmap")).expect("remove map");
    let e = call(&mut app, "session.load", json!({"name": "Gut"})).expect_err("missing map");
    assert_eq!(e.data.as_ref().expect("data")["kind"], "missing_content");
    assert_eq!(
        e.data.as_ref().expect("data")["issues"][0]["kind"],
        "MissingMap"
    );

    std::fs::create_dir_all(root.join("maps/Testmap")).expect("map back");
    std::fs::write(root.join("maps/Testmap/global.cfg"), "x").expect("cfg");
    app.world_mut().resource_mut::<NewGameSetup>().selected_map = "Other".into();
    let ok = call(&mut app, "session.load", json!({"name": "Gut"})).expect("load");
    assert_eq!(ok["started"], true);
    assert_eq!(
        app.world().resource::<NewGameSetup>().selected_map,
        "Testmap"
    );
    let list = call(&mut app, "session.saves", json!({})).expect("saves");
    assert_eq!(list["restore"]["state"], "loading");
    let busy = call(&mut app, "session.load", json!({"name": "Gut"})).expect_err("busy");
    assert_eq!(busy.code, code::BUSY);
}
