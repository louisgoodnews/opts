//! Integration test suite for vehicle kinematics, dynamics, and manifest parsing.

use opts_core::formats::vehicles::bus::{BusAxle, BusManifest, RegistrationMode};
use opts_sim::vehicle::inputs::GearPosition;
use opts_sim::VehicleSimulation;

/// Helper constructing a synthetic standard transit bus manifest.
fn make_test_manifest(mass_tonnes: Option<f32>, wheel_diameter: Option<f32>) -> BusManifest {
    BusManifest {
        friendly_name: None,
        description: Vec::new(),
        number_file: None,
        registration_mode: RegistrationMode::None,
        sound_cfg: None,
        sound_ai_cfg: None,
        model_cfg: None,
        paths_cfg: None,
        passengercabin_cfg: None,
        varnamelists: Vec::new(),
        stringvarnamelists: Vec::new(),
        scripts: Vec::new(),
        constfiles: Vec::new(),
        script_share: false,
        mass_tonnes,
        axles: vec![
            BusAxle {
                longitudinal_m: 3.25,
                max_width_m: Some(2.45),
                min_width_m: None,
                wheel_diameter_m: wheel_diameter,
                spring_rate: Some(280.0),
                max_force: Some(120.0),
                damping: Some(18.0),
                is_driven: false,
            },
            BusAxle {
                longitudinal_m: -2.60,
                max_width_m: Some(2.45),
                min_width_m: None,
                wheel_diameter_m: wheel_diameter,
                spring_rate: Some(340.0),
                max_force: Some(150.0),
                damping: Some(22.0),
                is_driven: true,
            },
        ],
        coupling_front_y: None,
        coupling_back_y: None,
        driver_cameras: Vec::new(),
        pax_cameras: Vec::new(),
        std_camera_index: None,
        outside_camera_center: None,
    }
}

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
