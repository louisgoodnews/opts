//! Drive controls: the driving commands, their gates, odometer and teleport.

use super::*;

#[test]
fn drive_controls_reach_the_vehicle_through_the_input_system() {
    let mut app = drive_app();
    assert!(inputs(&mut app).4, "starts parked");
    call(
        &mut app,
        "drive.set",
        json!({"control": "gear", "value": "drive"}),
    )
    .expect("gear");
    call(
        &mut app,
        "drive.set",
        json!({"control": "parking_brake", "value": false}),
    )
    .expect("pb");
    call(
        &mut app,
        "drive.set",
        json!({"control": "throttle", "value": 0.6, "hold_ms": 100}),
    )
    .expect("throttle");
    call(
        &mut app,
        "drive.set",
        json!({"control": "steer", "value": -0.4}),
    )
    .expect("steer");
    app.update();
    let (throttle, _, steer, gear, parked) = inputs(&mut app);
    assert!((throttle - 0.6).abs() < 1e-5, "throttle {throttle}");
    assert!((steer + 0.4).abs() < 1e-5, "steer {steer}");
    assert_eq!(gear, GearPosition::Drive);
    assert!(!parked);
    // 100 ms of simulation time later the hold has run out and the pedal decays on its own.
    for _ in 0..30 {
        app.update();
    }
    assert!(
        inputs(&mut app).0 < 0.2,
        "throttle released after its duration"
    );
    assert!(
        (inputs(&mut app).2 + 0.4).abs() < 1e-5,
        "an open-ended hold stays"
    );
    call(&mut app, "drive.release", json!({})).expect("release");
    for _ in 0..200 {
        app.update();
    }
    assert!(
        inputs(&mut app).2.abs() < 0.05,
        "steering returns after release"
    );
    let controls = call(&mut app, "drive.controls", json!({})).expect("controls");
    assert!(controls["controls"].as_array().unwrap().len() >= 5);
}

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

#[test]
fn odometer_teleport_reset_and_state() {
    let mut app = drive_app();
    let set_y = |app: &mut App, y: f32| {
        let mut q = app
            .world_mut()
            .query_filtered::<&mut VehicleController, With<PlayerVehicle>>();
        q.iter_mut(app.world_mut())
            .next()
            .unwrap()
            .sim
            .chassis
            .position = Vec2::new(0.0, y);
    };
    for i in 1..=10 {
        set_y(&mut app, i as f32);
        app.update();
    }
    let state = call(&mut app, "drive.state", json!({})).expect("state");
    assert!(
        (state["distance_m"].as_f64().unwrap() - 10.0).abs() < 0.5,
        "{state}"
    );
    assert!(
        state["route"].is_null() && state["stops"].is_null(),
        "timetable is not guessed"
    );
    // A jump is a placement, not driving.
    set_y(&mut app, 500.0);
    app.update();
    let jumped = call(&mut app, "drive.state", json!({})).expect("state");
    assert!((jumped["distance_m"].as_f64().unwrap() - 10.0).abs() < 0.5);
    // Teleport: absolute coordinates, standing still, odometer restarts.
    let t = call(
        &mut app,
        "vehicle.teleport",
        json!({"x": 200.0, "y": 100.0, "heading_deg": 90.0}),
    )
    .expect("tp");
    assert_eq!(t["ok"], true);
    app.update();
    let s = call(&mut app, "drive.state", json!({})).expect("state");
    assert!((s["x"].as_f64().unwrap() - 200.0).abs() < 0.01);
    assert!((s["y"].as_f64().unwrap() - 100.0).abs() < 0.01);
    assert!((s["heading_deg"].as_f64().unwrap() - 90.0).abs() < 0.01);
    assert!(s["distance_m"].as_f64().unwrap() < 0.01);
    assert!(s["speed_kmh"].as_f64().unwrap() < 0.01);
    // Reset: back to where the session placed the vehicle (the origin here).
    call(&mut app, "vehicle.reset", json!({})).expect("reset");
    app.update();
    let r = call(&mut app, "drive.state", json!({})).expect("state");
    assert!(
        r["x"].as_f64().unwrap().abs() < 0.01 && r["y"].as_f64().unwrap().abs() < 0.01,
        "{r}"
    );
}
