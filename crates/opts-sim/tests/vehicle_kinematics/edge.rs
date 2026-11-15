//! Zero and degenerate inputs, and the determinism of a long run.

use super::support::*;
use opts_sim::vehicle::inputs::GearPosition;
use opts_sim::VehicleSimulation;

#[test]
fn test_defensive_edge_cases_and_zero_inputs() {
    // 1. Missing mass fallback: None should default to ~11.5 tons
    let manifest_no_mass = make_test_manifest(None, None);
    let sim_no_mass = VehicleSimulation::from_manifest(&manifest_no_mass);
    assert!((sim_no_mass.chassis.total_mass_kg - 11_500.0).abs() < 1.0);

    // 2. Zero wheel diameter fallback: 0.0 should default to 0.98m
    let manifest_zero_wheel = make_test_manifest(Some(10.0), Some(0.0));
    let sim_zero_wheel = VehicleSimulation::from_manifest(&manifest_zero_wheel);
    assert!((sim_zero_wheel.axles[0].config.wheel_diameter_m - 0.98).abs() < 1e-3);

    // 3. Zero steering angle: must travel strictly straight without NaN or lateral drift
    let mut sim_straight = VehicleSimulation::new_standard_bus(11_500.0, 5.85);
    sim_straight.inputs.set_parking_brake(false);
    sim_straight.inputs.set_gear(GearPosition::Drive);
    sim_straight.inputs.set_throttle(1.0);
    sim_straight.inputs.set_steering(0.0);

    for _ in 0..180 {
        sim_straight.step(1.0 / 60.0);
    }

    assert_eq!(sim_straight.chassis.heading_rad, 0.0);
    assert_eq!(sim_straight.chassis.position.x, 0.0);
    assert!(sim_straight.chassis.position.y > 0.0);
    assert!(!sim_straight.chassis.forward_speed_m_s().is_nan());
}

#[test]
fn test_simulation_long_run_determinism() {
    let manifest = make_test_manifest(Some(12.5), Some(0.96));
    let mut sim1 = VehicleSimulation::from_manifest(&manifest);
    sim1.inputs.set_gear(GearPosition::Drive);
    sim1.inputs.set_parking_brake(false);

    let mut sim2 = sim1.clone();

    // Run 300 steps with dynamic control sequences
    for step in 0..300 {
        let t = step as f32 * 0.016;
        let throttle = 0.5 + 0.3 * (t * 2.0).sin();
        let steer = 0.4 * (t * 1.5).cos();

        sim1.inputs.set_throttle(throttle);
        sim1.inputs.set_steering(steer);

        sim2.inputs.set_throttle(throttle);
        sim2.inputs.set_steering(steer);

        sim1.step(1.0 / 60.0);
        sim2.step(1.0 / 60.0);

        assert_eq!(
            sim1.chassis.position, sim2.chassis.position,
            "Positions diverged at step {step}"
        );
        assert_eq!(
            sim1.chassis.local_velocity, sim2.chassis.local_velocity,
            "Velocities diverged at step {step}"
        );
        assert_eq!(
            sim1.chassis.heading_rad, sim2.chassis.heading_rad,
            "Headings diverged at step {step}"
        );
    }
}
