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

mod gates;

mod odometer;
