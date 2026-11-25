//! The pedals and the script's torque and brake force give back the desired acceleration.

use opts_scripted_drive::{pedals, script_accel};

const MASS: f64 = 12_000.0;
const RADIUS: f64 = 0.5;
const MAX_ACCEL: f64 = 1.0;
const MAX_DECEL: f64 = 2.0;

#[test]
fn accelerating_uses_only_the_throttle() {
    assert_eq!(pedals(0.5, MAX_ACCEL, MAX_DECEL), (0.5, 0.0));
    assert_eq!(pedals(3.0, MAX_ACCEL, MAX_DECEL), (1.0, 0.0));
}

#[test]
fn braking_uses_only_the_brake() {
    assert_eq!(pedals(-1.0, MAX_ACCEL, MAX_DECEL), (0.0, 0.5));
    assert_eq!(pedals(-9.0, MAX_ACCEL, MAX_DECEL), (0.0, 1.0));
}

#[test]
fn drive_and_brake_forces_combine_into_an_acceleration() {
    // Full drive torque 6000 N·m on 0.5 m wheels is 12 kN: 1 m/s² on 12 t.
    assert!((script_accel(6000.0, 0.0, MASS, RADIUS) - 1.0).abs() < 1e-9);
    // A 6 kN brake force takes 0.5 m/s² off that.
    assert!((script_accel(6000.0, 6000.0, MASS, RADIUS) - 0.5).abs() < 1e-9);
    assert!((script_accel(0.0, 12_000.0, MASS, RADIUS) + 1.0).abs() < 1e-9);
}

#[test]
fn the_pedals_reproduce_the_desired_acceleration() {
    // The script gives the full drive torque of the vehicle at full throttle.
    let full_torque = MAX_ACCEL * RADIUS * MASS;
    for desired in [-1.8, -0.7, 0.0, 0.3, 0.9] {
        let (throttle, brake) = pedals(desired, MAX_ACCEL, MAX_DECEL);
        let back = script_accel(
            throttle * full_torque,
            brake * MAX_DECEL * MASS,
            MASS,
            RADIUS,
        );
        assert!((back - desired).abs() < 1e-9, "{desired} -> {back}");
    }
}
