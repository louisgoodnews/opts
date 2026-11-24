//! `vehicle.fuel` over the protocol: the level, the refill and the refusal of a refuel while the
//! vehicle moves (M-0123). The inline tests of `control_fuel` moved here.

use super::*;
use opts_sim::vehicle::fuel::{FuelKind, FuelMode};

/// The drive app with a diesel model on the player's vehicle, at `speed` m/s forward.
fn fuelled_app(speed: f32) -> App {
    let mut app = drive_app();
    let mut q = app
        .world_mut()
        .query_filtered::<&mut VehicleController, With<PlayerVehicle>>();
    let mut controller = q.iter_mut(app.world_mut()).next().expect("player vehicle");
    controller
        .sim
        .ensure_fuel(FuelKind::Diesel, 250.0, FuelMode::Realistic);
    controller.sim.chassis.local_velocity.y = speed;
    app
}

fn level(app: &mut App) -> f64 {
    call(app, "vehicle.fuel", json!({})).expect("fuel")["level"]
        .as_f64()
        .expect("level")
}

#[test]
fn the_level_is_set_and_reported() {
    let mut app = fuelled_app(0.0);
    let d = call(&mut app, "vehicle.fuel", json!({ "level": 0.25 })).expect("set");
    assert!((d["level"].as_f64().unwrap() - 0.25).abs() < 1e-6);
    assert_eq!(d["mode"], "realistic");
    assert_eq!(d["low"], false);
    assert_eq!(d.get("refused"), None, "no refused member without a refuel");
    let d = call(&mut app, "vehicle.fuel", json!({ "level": 0.05 })).expect("low");
    assert_eq!(d["low"], true);
}

#[test]
fn refuel_fills_the_store_and_a_bad_level_is_refused() {
    let mut app = fuelled_app(0.0);
    call(&mut app, "vehicle.fuel", json!({ "level": 0.0 })).expect("empty");
    let d = call(&mut app, "vehicle.fuel", json!({ "refuel": true })).expect("refuel");
    assert_eq!(d["refused"], false);
    assert!((level(&mut app) - 1.0).abs() < 1e-6);
    let e = call(&mut app, "vehicle.fuel", json!({ "level": 1.5 })).expect_err("bad level");
    assert_eq!(e.code, code::INVALID_PARAMS);
}

#[test]
fn an_empty_request_changes_nothing() {
    let mut app = fuelled_app(0.0);
    let d = call(&mut app, "vehicle.fuel", json!({})).expect("read");
    assert!((d["level"].as_f64().unwrap() - 1.0).abs() < 1e-6);
    assert_eq!(d.get("refused"), None);
}

#[test]
fn a_refuel_while_moving_is_refused_in_the_result() {
    for speed in [5.0, -5.0] {
        let mut app = fuelled_app(0.0);
        call(&mut app, "vehicle.fuel", json!({ "level": 0.0 })).expect("empty");
        app.world_mut()
            .query_filtered::<&mut VehicleController, With<PlayerVehicle>>()
            .iter_mut(app.world_mut())
            .next()
            .expect("player vehicle")
            .sim
            .chassis
            .local_velocity
            .y = speed;
        let d = call(&mut app, "vehicle.fuel", json!({ "refuel": true })).expect("answered");
        assert_eq!(
            d["refused"], true,
            "{speed} m/s: refused, not a silent success"
        );
        assert!(level(&mut app).abs() < 1e-6, "{speed} m/s: store unchanged");
    }
}

#[test]
fn the_refusal_threshold_holds_over_the_protocol() {
    for (speed, refused) in [(0.49, false), (0.51, true), (-0.49, false), (-0.51, true)] {
        let mut app = fuelled_app(0.0);
        call(&mut app, "vehicle.fuel", json!({ "level": 0.0 })).expect("empty");
        app.world_mut()
            .query_filtered::<&mut VehicleController, With<PlayerVehicle>>()
            .iter_mut(app.world_mut())
            .next()
            .expect("player vehicle")
            .sim
            .chassis
            .local_velocity
            .y = speed;
        let d = call(&mut app, "vehicle.fuel", json!({ "refuel": true })).expect("answered");
        assert_eq!(d["refused"], refused, "{speed} m/s");
    }
}
