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

mod load;
