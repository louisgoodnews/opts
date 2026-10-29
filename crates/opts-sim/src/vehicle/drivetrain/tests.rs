//! Unit tests for drivetrain models and forces.

use super::*;

#[test]
fn test_engine_torque_curve() {
    let engine = EngineSpec::default();
    // At 0 throttle, torque must be 0
    assert_eq!(engine.compute_torque(1200.0, 0.0), 0.0);

    // At full throttle at peak RPM (1200), torque should approach peak torque (1100 Nm)
    let peak = engine.compute_torque(1200.0, 1.0);
    assert!((peak - 1100.0).abs() < 10.0);

    // At redline, torque drops
    let redline = engine.compute_torque(2200.0, 1.0);
    assert!(redline < peak);
}

#[test]
fn test_aerodynamic_drag_scaling() {
    let aero = AerodynamicSpec::default();
    let drag_10 = aero.compute_drag_force(10.0);
    let drag_20 = aero.compute_drag_force(20.0);

    // Drag is proportional to v^2; doubling speed quadruples drag force
    let ratio = drag_20 / drag_10;
    assert!((ratio - 4.0).abs() < 1e-3);
}

#[test]
fn test_drivetrain_neutral_and_drive() {
    let mut model = DrivetrainModel::new();
    let mut inputs = VehicleInputs::new();
    inputs.set_parking_brake(false);
    inputs.set_throttle(1.0);
    inputs.set_gear(GearPosition::Neutral);

    // In Neutral, tractive force must be 0
    let forces = model.step(&inputs, 0.0, 0.49, 11500.0);
    assert_eq!(forces.tractive_force_n, 0.0);

    // In Drive, forward tractive force must be positive
    inputs.set_gear(GearPosition::Drive);
    let forces_drive = model.step(&inputs, 0.0, 0.49, 11500.0);
    assert!(forces_drive.tractive_force_n > 5000.0);
}
