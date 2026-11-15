use super::*;

fn params(catalogue_id: &str, repaint: &str, setvars: Vec<(String, i32)>) -> SpawnParams {
    SpawnParams {
        catalogue_id: catalogue_id.into(),
        name: "x".into(),
        bus_file: "/x.bus".into(),
        repaint: repaint.into(),
        setvars,
        entry: EntrySel::Default,
        formation: None,
    }
}

#[test]
fn in_a_session_the_active_vehicle_is_what_gets_saved() {
    let mut f = fixture("session-save");
    let mut registry = SessionVehicles::default();
    let id = registry.register_alive(
        params("solaris", "Blue", vec![("display_ext".into(), 5)]),
        Entity::PLACEHOLDER,
        None,
        None,
        None,
    );
    registry.set_active(Some(id));
    f.app.insert_resource(registry);
    let saved = call(&mut f.app, "config.save", json!({ "name": "Second" })).expect("saves");
    assert_eq!(saved["configuration"]["vehicle"], "solaris");
    assert_eq!(saved["configuration"]["repaint"], "Blue");
    assert_eq!(saved["configuration"]["setvars"]["display_ext"], 5);
}

#[test]
fn spawning_a_configuration_needs_a_session_and_a_known_configuration() {
    let mut f = fixture("spawn-refusals");
    call(&mut f.app, "config.save", json!({ "name": "Line 33" })).expect("saves");
    let e = call(&mut f.app, "config.spawn", json!({ "id": "line-33" })).expect_err("no session");
    assert_eq!(e.code, code::NOT_READY);
    f.app.insert_resource(SessionVehicles::default());
    let e = call(&mut f.app, "config.spawn", json!({ "id": "nope" })).expect_err("unknown");
    assert_eq!(e.code, code::NOT_FOUND);
    let e = call(&mut f.app, "config.spawn", json!({ "id": "line-33" })).expect_err("no world");
    assert_eq!(e.code, code::NOT_READY);
    let e = call(&mut f.app, "config.spawn", json!({})).expect_err("no id");
    assert_eq!(e.code, code::INVALID_PARAMS);
}
