//! The drive curve: interpolation, clamping, the acceleration law and the run-up.

use opts_kinematic::DriveCurve;

#[test]
fn torque_is_linear_between_samples_and_clamped_outside() {
    let c = DriveCurve::new(vec![(50.0, 100.0), (0.0, 1000.0), (100.0, 0.0)]);
    assert_eq!(c.torque_at(-5.0), 1000.0);
    assert_eq!(c.torque_at(25.0), 550.0);
    assert_eq!(c.torque_at(75.0), 50.0);
    assert_eq!(c.torque_at(500.0), 0.0);
}

#[test]
fn an_empty_curve_gives_no_torque() {
    assert_eq!(DriveCurve::new(Vec::new()).torque_at(10.0), 0.0);
}

#[test]
fn acceleration_is_torque_over_wheel_radius_times_mass() {
    let c = DriveCurve::new(vec![(0.0, 6000.0)]);
    assert!((c.acceleration_at(0.0, 12_000.0, 0.5) - 1.0).abs() < 1e-6);
}

#[test]
fn a_constant_torque_reaches_the_target_at_the_analytic_time() {
    // a = 1 m/s² from rest: 50 km/h (13.89 m/s) at 13.89 s, distance v²/(2a) = 96.5 m.
    let c = DriveCurve::new(vec![(0.0, 6000.0)]);
    let (t, x) = c
        .run_up(50.0 / 3.6, 12_000.0, 0.5, 1.0 / 600.0, 60.0)
        .expect("reaches 50 km/h");
    assert!((t - 13.89).abs() < 0.02, "{t}");
    assert!((x - 96.5).abs() < 0.5, "{x}");
}

#[test]
fn a_vehicle_with_no_torque_never_reaches_the_target() {
    let c = DriveCurve::new(vec![(0.0, 0.0)]);
    assert!(c.run_up(10.0, 12_000.0, 0.5, 0.1, 30.0).is_none());
}
