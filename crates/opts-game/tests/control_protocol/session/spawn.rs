//! Listing and spawning session vehicles through the control protocol.


use super::*;


#[test]
pub(super) fn session_vehicles_lists_the_registry_and_the_place_label() {
    let mut app = session_app();
    let empty = call(&mut app, "session.vehicles", json!({})).expect("empty");
    assert_eq!(empty["count"], 0);
    assert_eq!(empty["active"], Value::Null);
    assert_eq!(empty["place_label"], Value::Null);
    let a = add_vehicle(&mut app, "Alpha", "Depot", 0.0);
    let b = add_vehicle(&mut app, "Beta", "Stop", 50.0);
    call(&mut app, "session.switch", json!({ "id": a })).expect("switch");
    app.update();
    app.update();
    let list = call(&mut app, "session.vehicles", json!({})).expect("list");
    assert_eq!(list["count"], 2);
    assert_eq!(list["active"], a);
    assert_eq!(list["place_label"], "Depot");
    let first = &list["vehicles"][0];
    assert_eq!(
        (
            first["name"].as_str(),
            first["spawn_entry_name"].as_str(),
            first["lifecycle"].as_str()
        ),
        (Some("Alpha"), Some("Depot"), Some("alive"))
    );
    assert_eq!(list["vehicles"][1]["id"], b);
    assert_eq!(list["vehicles"][1]["active"], false);
}

#[test]
pub(super) fn session_spawn_queues_a_request_and_validates_its_parameters() {
    let dir = scratch("session-spawn");
    let mut app = base_app(vec![dir.clone()]);
    app.add_plugins(SessionVehiclesPlugin);
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::InGame);
    for _ in 0..3 {
        app.update();
    }
    let bus = dir.join("Test_Bus.bus");
    std::fs::write(&bus, "x").expect("bus file");
    let err = call(&mut app, "session.spawn", json!({})).expect_err("vehicle is required");
    assert_eq!(err.code, code::INVALID_PARAMS);
    let err = call(
        &mut app,
        "session.spawn",
        json!({ "vehicle": "No Such Bus" }),
    )
    .expect_err("unknown vehicle");
    assert_eq!(err.code, code::NOT_FOUND);
    let err = call(
        &mut app,
        "session.spawn",
        json!({ "vehicle": bus.display().to_string(), "setvars": [1] }),
    )
    .expect_err("setvars is an object");
    assert_eq!(err.code, code::INVALID_PARAMS);
    let ok = call(
        &mut app,
        "session.spawn",
        json!({
            "vehicle": bus.display().to_string(),
            "repaint": "Red",
            "setvars": { "Klima": 1 },
            "entry": "Depot",
        }),
    )
    .expect("queued");
    assert_eq!(ok["queued"], true);
    let list = call(&mut app, "session.vehicles", json!({})).expect("list");
    let v = &list["vehicles"][0];
    assert_eq!(v["id"], ok["id"]);
    assert_eq!(v["lifecycle"], "spawning");
    assert_eq!(v["repaint"], "Red");
    assert_eq!(v["setvars"][0]["name"], "Klima");
    assert_eq!(v["entry"]["name"], "Depot");
    // A request that never existed in the world can be cancelled.
    let gone = call(&mut app, "session.despawn", json!({ "id": ok["id"] })).expect("cancel");
    assert_eq!(gone["ok"], true);
    let list = call(&mut app, "session.vehicles", json!({})).expect("list");
    assert_eq!(list["vehicles"][0]["lifecycle"], "despawned");
    let _ = std::fs::remove_dir_all(dir);
}
