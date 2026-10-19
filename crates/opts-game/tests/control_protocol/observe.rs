//! wait_for, snapshot and screen.

use super::*;

#[test]
fn wait_for_holds_on_conditions_and_times_out_with_the_last_value() {
    let mut app = drive_app();
    // Paused clock: the condition holds at once.
    app.world_mut().resource_mut::<Time<Virtual>>().pause();
    let ok = call(&mut app, "ui.wait_for", json!({"until": "paused"})).expect("paused");
    assert_eq!(ok["ok"], true);
    // A distance that is not driven times out with what was observed.
    let e = call(
        &mut app,
        "ui.wait_for",
        json!({"until": "distance_above", "number": 100.0, "timeout_ms": 60}),
    );
    // The waiter polls once per frame; give it real time to pass the deadline.
    let e = match e {
        Err(e) => e,
        Ok(v) => panic!("should time out: {v}"),
    };
    assert_eq!(e.code, code::TIMEOUT);
    assert_eq!(e.structured().data.expect("data")["name"], "TIMEOUT");
    // Invalid conditions are rejected up front, with the valid ones listed.
    let bad = call(&mut app, "ui.wait_for", json!({"until": "soon"})).expect_err("bad");
    assert_eq!(bad.code, code::INVALID_PARAMS);
    assert!(bad.data.expect("data")["valid"].as_array().unwrap().len() > 10);
    let missing =
        call(&mut app, "ui.wait_for", json!({"until": "speed_above"})).expect_err("number");
    assert_eq!(missing.code, code::INVALID_PARAMS);
    // app_state is read case-insensitively.
    let s = call(
        &mut app,
        "ui.wait_for",
        json!({"until": "app_state", "value": "loading"}),
    )
    .expect("state");
    assert_eq!(s["ok"], true);
}

#[test]
fn snapshot_collects_state_screen_drive_and_warnings_into_a_file() {
    let dir = scratch("snapshot");
    let mut app = base_app(vec![dir.clone()]);
    app.add_plugins(OptsVehiclePlugin);
    app.world_mut().spawn((
        VehicleController::standard_bus(12_000.0, 5.9),
        PlayerVehicle,
    ));
    app.update();
    let path = dir.join("snap.json");
    let v = call(
        &mut app,
        "snapshot",
        json!({"path": path.display().to_string()}),
    )
    .expect("snapshot");
    for key in ["state", "screen", "drive", "camera", "sim", "warnings"] {
        assert!(v.get(key).is_some(), "snapshot has `{key}`: {v}");
    }
    assert_eq!(v["screen"]["app_state"], "Loading");
    let on_disk: Value =
        serde_json::from_str(&std::fs::read_to_string(&path).expect("file")).expect("json");
    assert_eq!(on_disk["drive"]["gear"], v["drive"]["gear"]);
    // Writing outside the allowed directories is refused.
    let e = call(&mut app, "snapshot", json!({"path": "/proc/nope.json"})).expect_err("forbidden");
    assert!(e.code == code::FORBIDDEN_PATH || e.code == code::NOT_FOUND);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn screen_reports_state_without_dialogs() {
    let mut app = base_app(Vec::new());
    let s = call(&mut app, "ui.screen", json!({})).expect("screen");
    assert_eq!(s["app_state"], "Loading");
    assert_eq!(s["modal_open"], false);
    assert!(s["dialogs"].as_array().unwrap().is_empty());
    app.insert_resource(ModalActive(true));
    let s = call(&mut app, "ui.screen", json!({})).expect("screen");
    assert_eq!(s["modal_open"], true, "the report modal counts");
}
