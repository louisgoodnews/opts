//! The odometer, a placement is not driving, teleport and reset.

use super::*;

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
