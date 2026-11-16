//! Drive controls behind the input gates: modal, free roam, held values and bad values, and no vehicle.

use super::*;

#[test]
fn drive_controls_are_refused_by_the_same_gates_as_keys() {
    let mut app = drive_app();
    // A dialog or the vehicle menu is open.
    app.insert_resource(ModalActive(true));
    let e = call(
        &mut app,
        "drive.set",
        json!({"control": "throttle", "value": 1.0}),
    )
    .expect_err("gated");
    assert_eq!(e.code, code::GATED);
    assert_eq!(e.structured().data.expect("data")["name"], "GATED");
    app.insert_resource(ModalActive(false));
    // Free roam: the vehicle takes no input.
    app.world_mut().resource_mut::<CameraState>().mode = CameraMode::FreeRoam;
    let e = call(
        &mut app,
        "drive.set",
        json!({"control": "gear", "value": "drive"}),
    )
    .expect_err("free roam");
    assert_eq!(e.code, code::GATED);
    app.world_mut().resource_mut::<CameraState>().mode = CameraMode::External;
    // A held value is dropped when a gate closes, so nothing fires when it reopens.
    call(
        &mut app,
        "drive.set",
        json!({"control": "brake", "value": 1.0}),
    )
    .expect("brake");
    app.insert_resource(ModalActive(true));
    app.update();
    assert!(!app.world().resource::<RemoteControls>().is_active());
    app.insert_resource(ModalActive(false));
    app.update();
    assert!(
        inputs(&mut app).1 < 0.01,
        "the held brake did not come back"
    );
    // Bad values are invalid params, not gates.
    let bad = call(
        &mut app,
        "drive.set",
        json!({"control": "throttle", "value": 2}),
    )
    .expect_err("range");
    assert_eq!(bad.code, code::INVALID_PARAMS);
    let bad = call(
        &mut app,
        "drive.set",
        json!({"control": "gear", "value": "sideways"}),
    )
    .expect_err("gear");
    assert_eq!(bad.code, code::INVALID_PARAMS);
}

#[test]
fn without_a_vehicle_driving_commands_say_not_ready() {
    let mut app = base_app(Vec::new());
    for (m, p) in [
        ("drive.set", json!({"control": "throttle", "value": 0.5})),
        ("drive.trigger", json!({"name": "horn"})),
        ("drive.state", json!({})),
        ("drive.controls", json!({})),
        ("vehicle.reset", json!({})),
    ] {
        let e = call(&mut app, m, p).expect_err(m);
        assert_eq!(e.code, code::NOT_READY, "{m}");
    }
}
