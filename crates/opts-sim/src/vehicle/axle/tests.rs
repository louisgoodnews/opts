//! Unit tests for `super` (`axle.rs`).
use super::*;

#[test]
fn test_ackermann_straight_line_safe() {
    // Zero steering angle must return (0.0, 0.0) without NaN or panic
    let (left, right) = AxleState::compute_ackermann_angles(5.8, 2.45, 0.0);
    assert_eq!(left, 0.0);
    assert_eq!(right, 0.0);
    assert!(!left.is_nan());
    assert!(!right.is_nan());
}

#[test]
fn test_ackermann_steering_geometry() {
    let wheelbase = 5.85; // Standard 12m bus wheelbase
    let track_width = 2.45;
    let steer_input = 0.5; // ~28.6 degrees right turn

    let (left_angle, right_angle) =
        AxleState::compute_ackermann_angles(wheelbase, track_width, steer_input);

    // For a right turn: right wheel is inner, left wheel is outer.
    // Inner wheel must turn sharper than outer wheel: right_angle > left_angle > 0
    assert!(
        right_angle > left_angle,
        "Right inner wheel must turn sharper than left outer"
    );
    assert!(left_angle > 0.0);
    assert!(right_angle > 0.0);
}

#[test]
fn test_wheel_rotation_advancement() {
    let mut axle = AxleState::from_manifest(
        &BusAxle {
            longitudinal_m: 3.2,
            max_width_m: Some(2.45),
            min_width_m: None,
            wheel_diameter_m: Some(1.0), // r = 0.5m
            spring_rate: None,
            max_force: None,
            damping: None,
            is_driven: false,
        },
        true,
    );

    // At 10 m/s with r = 0.5m, ang_vel = 20 rad/s
    axle.update_rotation(10.0, 0.1);
    assert!((axle.left_wheel.angular_velocity_rad_s - 20.0).abs() < 1e-3);
    assert!((axle.left_wheel.rotation_rad - 2.0).abs() < 1e-3);
}
