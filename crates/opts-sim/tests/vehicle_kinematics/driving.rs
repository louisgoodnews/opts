//! Acceleration, the brake stop cycle and the turning circle.

use super::support::*;
use opts_sim::vehicle::inputs::GearPosition;
use opts_sim::VehicleSimulation;

#[test]
fn test_driving_acceleration_and_brake_stop_cycle() {
    let manifest = make_test_manifest(Some(11.8), Some(0.98));
    let mut sim = VehicleSimulation::from_manifest(&manifest);

    // Initial state: stationary in Neutral with parking brake
    assert_eq!(sim.chassis.forward_speed_km_h(), 0.0);
    assert!(sim.inputs.is_parking_brake_engaged());

    // 1. Shift to Drive, release handbrake, apply full throttle
    sim.inputs.set_gear(GearPosition::Drive);
    sim.inputs.set_parking_brake(false);
    sim.inputs.set_throttle(1.0);

    // Accelerate for 3.0 seconds (180 steps at 60 Hz)
    for _ in 0..180 {
        sim.step(1.0 / 60.0);
    }

    let top_speed = sim.chassis.forward_speed_km_h();
    assert!(
        top_speed > 20.0,
        "Vehicle should accelerate beyond 20 km/h in 3s (actual: {top_speed:.1} km/h)"
    );
    assert!(
        sim.chassis.position.y > 10.0,
        "Vehicle should have traveled forward"
    );

    // 2. Release throttle and apply 100% service brake
    sim.inputs.set_throttle(0.0);
    sim.inputs.set_service_brake(1.0);

    // Brake for 2.5 seconds (150 steps)
    for _ in 0..150 {
        sim.step(1.0 / 60.0);
    }

    // Vehicle must come to a complete standstill without reversing
    assert_eq!(
        sim.chassis.forward_speed_m_s(),
        0.0,
        "Vehicle must halt completely at 0.0 m/s"
    );
    assert_eq!(
        sim.chassis.local_acceleration.y, 0.0,
        "Acceleration should clamp to 0.0 when stopped"
    );

    // 3. Set parking brake
    sim.inputs.set_service_brake(0.0);
    sim.inputs.set_parking_brake(true);
    for _ in 0..60 {
        sim.step(1.0 / 60.0);
    }
    assert_eq!(sim.chassis.forward_speed_m_s(), 0.0);
}

#[test]
fn test_turning_circle_and_yaw_motion() {
    let manifest = make_test_manifest(Some(11.5), Some(0.98));
    let mut sim = VehicleSimulation::from_manifest(&manifest);

    sim.inputs.set_gear(GearPosition::Drive);
    sim.inputs.set_parking_brake(false);
    sim.inputs.set_throttle(0.5);
    sim.inputs.set_steering(0.6); // Sharp right turn

    // Drive for 2.0 seconds
    for _ in 0..120 {
        sim.step(1.0 / 60.0);
    }

    // Vehicle heading must rotate clockwise (positive yaw in radians)
    assert!(
        sim.chassis.heading_rad > 0.1,
        "Heading must rotate clockwise during right turn (actual: {:.3} rad)",
        sim.chassis.heading_rad
    );

    // Vehicle position must describe a curved arc (both X and Y positions change)
    assert!(
        sim.chassis.position.x > 0.0,
        "Trajectory must curve to the right"
    );
    assert!(
        sim.chassis.position.y > 0.0,
        "Trajectory must advance forward"
    );

    // Inner wheel must have turned sharper than outer wheel
    let front_axle = &sim.axles[0];
    assert!(
        front_axle.right_wheel.steer_angle_rad > front_axle.left_wheel.steer_angle_rad,
        "Ackermann inner right wheel angle must exceed outer left wheel angle"
    );
}
