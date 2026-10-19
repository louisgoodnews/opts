//! Session save and load.

use super::*;

/// A running, loaded session (no map: the setup's map is the folder) with a store in `tag`.
fn save_app(tag: &str) -> App {
    let mut app = base_app(Vec::new());
    app.add_plugins(OptsVehiclePlugin);
    app.init_resource::<NewGameSetup>()
        .init_resource::<InGameSessionInfo>()
        .init_resource::<opts_game::session_vehicles::SessionVehicles>()
        .init_resource::<opts_game::pause::SessionStats>()
        .init_resource::<opts_game::pause::Pause>()
        .init_resource::<opts_game::save::PlayClock>()
        .init_resource::<opts_game::save::LastRestore>()
        .init_resource::<opts_game::save::SaveRegistry>()
        .init_resource::<opts_bevy::CameraAvailability>()
        .init_resource::<SimulationLoadState>()
        .insert_resource(SaveStorage(Some(SaveStore::new(
            scratch(tag).join("saves"),
        ))));
    opts_game::save::sections::register(&mut app);
    app.world_mut()
        .resource_mut::<SimulationLoadState>()
        .is_complete = true;
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::InGame);
    for _ in 0..3 {
        app.update();
    }
    app
}

#[test]
fn session_save_list_rename_delete_and_the_command_table() {
    let mut app = save_app("save-basic");
    let empty = call(&mut app, "session.saves", json!({})).expect("saves");
    assert_eq!(empty["saves"], json!([]));
    assert_eq!(empty["running"], true);
    assert_eq!(empty["restore"]["state"], "idle");
    let ids: Vec<&str> = empty["sections"]
        .as_array()
        .expect("sections")
        .iter()
        .filter_map(|s| s["id"].as_str())
        .collect();
    for id in [
        "session",
        "time",
        "weather",
        "session_vehicles",
        "camera",
        "counters",
    ] {
        assert!(ids.contains(&id), "{id} not registered: {ids:?}");
    }

    let saved = call(&mut app, "session.save", json!({"name": "Erste Fahrt"})).expect("save");
    assert_eq!(saved["id"], "erste-fahrt");
    assert_eq!(saved["overwritten"], false);
    let again = call(&mut app, "session.save", json!({"name": "erste fahrt"})).expect_err("exists");
    assert_eq!(again.data.as_ref().expect("data")["kind"], "exists");
    let over = call(
        &mut app,
        "session.save",
        json!({"name": "Erste Fahrt", "overwrite": true}),
    )
    .expect("overwrite");
    assert_eq!(over["overwritten"], true);
    let empty_name = call(&mut app, "session.save", json!({"name": "  "})).expect_err("name");
    assert_eq!(empty_name.code, code::INVALID_PARAMS);

    let list = call(&mut app, "session.saves", json!({})).expect("saves");
    assert_eq!(list["saves"].as_array().map(Vec::len), Some(1));
    assert_eq!(list["saves"][0]["name"], "Erste Fahrt");
    assert!(
        list["saves"][0]["sections"]
            .as_array()
            .expect("sections")
            .len()
            >= 6
    );

    call(
        &mut app,
        "session.rename_save",
        json!({"name": "Erste Fahrt", "new_name": "Umbenannt"}),
    )
    .expect("rename");
    let list = call(&mut app, "session.saves", json!({})).expect("saves");
    assert_eq!(list["saves"][0]["name"], "Umbenannt");
    assert_eq!(list["saves"][0]["id"], "erste-fahrt");

    let missing =
        call(&mut app, "session.delete_save", json!({"name": "nope"})).expect_err("missing");
    assert_eq!(missing.code, code::NOT_FOUND);
    call(
        &mut app,
        "session.delete_save",
        json!({"name": "umbenannt"}),
    )
    .expect("delete");
    let list = call(&mut app, "session.saves", json!({})).expect("saves");
    assert_eq!(list["saves"], json!([]));
}

#[test]
fn session_save_needs_a_running_drive() {
    let mut app = save_app("save-idle");
    app.world_mut()
        .resource_mut::<SimulationLoadState>()
        .is_complete = false;
    let e = call(&mut app, "session.save", json!({"name": "x"})).expect_err("loading");
    assert_eq!(e.code, code::NOT_READY);
}

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
