//! Unit tests of the vehicle simulation: consist stepping, serialisation and teleport.

use super::*;

#[test]
fn the_consist_is_stepped_with_the_chassis_and_stays_hitched() {
    let mut sim = articulated();
    for _ in 0..600 {
        sim.step(1.0 / 64.0);
    }
    let alpha = sim.consist_alpha_rad(0).unwrap();
    assert!(alpha.abs() > 0.05 && alpha.abs() <= 52.5_f64.to_radians());
    let unit = &sim.consist[0];
    let (pos, heading) = sim.chassis_pose();
    let hitch = unit.hitch(pos, heading);
    let front = unit.origin
        + DVec2::new(unit.heading.cos(), -unit.heading.sin()) * unit.front_local.x
        + DVec2::new(unit.heading.sin(), unit.heading.cos()) * unit.front_local.y;
    assert!((front - hitch).length() < 1e-6);
    assert_eq!(sim.consist_alpha_rad(1), None);
}

/// Design V4: a consist driven into a circle, serialised, restored and driven on continues
/// exactly like the unbroken run (the acceptance asks for 1 mm and 0.01 degrees).
#[test]
fn a_serialised_consist_resumes_like_the_unbroken_run() {
    let mut a = articulated();
    for _ in 0..400 {
        a.step(1.0 / 64.0);
    }
    let bytes = bincode::serialize(&a).unwrap();
    let mut b: VehicleSimulation = bincode::deserialize(&bytes).unwrap();
    assert_eq!(a, b, "the restored state equals the saved one");
    for _ in 0..200 {
        a.step(1.0 / 64.0);
        b.step(1.0 / 64.0);
    }
    assert_eq!(a, b);
    let d = (a.consist[0].origin - b.consist[0].origin).length();
    let dalpha = (a.consist_alpha_rad(0).unwrap() - b.consist_alpha_rad(0).unwrap()).abs();
    assert!(d < 1e-3 && dalpha.to_degrees() < 0.01);
    assert!(
        a.consist_alpha_rad(0).unwrap().abs() > 0.05,
        "it is in a curve"
    );
}

#[test]
fn a_state_saved_without_a_consist_still_loads() {
    // `consist` and `teleports` are `serde(default)`; a vehicle of that shape round-trips.
    let solo = VehicleSimulation::default();
    let back: VehicleSimulation =
        bincode::deserialize(&bincode::serialize(&solo).unwrap()).unwrap();
    assert!(back.consist.is_empty());
    assert_eq!(solo, back);
}

#[test]
fn teleport_hangs_the_consist_straight_with_no_velocity() {
    let mut sim = articulated();
    for _ in 0..400 {
        sim.step(1.0 / 64.0);
    }
    assert!(sim.consist_alpha_rad(0).unwrap().abs() > 0.05);
    sim.teleport(Vec2::new(500.0, -300.0), 1.2);
    assert_eq!(sim.teleports, 1);
    assert!(sim.consist_alpha_rad(0).unwrap().abs() < 1e-12);
    // Standing still after the jump: the unit does not move.
    sim.inputs.set_throttle(0.0);
    sim.inputs.set_steering(0.0);
    sim.chassis.local_velocity = Vec2::ZERO;
    sim.chassis.yaw_rate_rad_s = 0.0;
    let before = sim.consist[0].origin;
    sim.step(1.0 / 64.0);
    assert!((sim.consist[0].origin - before).length() < 1e-6);
    assert!(sim.consist_alpha_rad(0).unwrap().abs() < 1e-6);
}
