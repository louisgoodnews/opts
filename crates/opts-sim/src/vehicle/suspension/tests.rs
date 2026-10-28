//! Unit tests for `super` (`suspension.rs`).

use super::*;

#[test]
fn test_static_load_sum_equals_weight() {
    let susp = SuspensionModel::new(1.15, 0.0);
    let mass = 12_000.0; // 12 metric tons
    let total_weight = mass * 9.81;

    let axle_positions = [3.2, -2.6]; // Front axle +3.2m, Rear axle -2.6m
    let loads = susp.compute_static_loads(&axle_positions, mass);

    let sum: f32 = loads.iter().sum();
    assert!(
        (sum - total_weight).abs() < 1.0,
        "Sum of axle loads must equal total weight"
    );
    assert!(loads[0] > 0.0);
    assert!(loads[1] > 0.0);
}

#[test]
fn test_dynamic_load_transfer() {
    let susp = SuspensionModel::new(1.15, 0.0);
    let mass = 12_000.0;
    let axle_positions = [3.2, -2.6];
    let static_loads = susp.compute_static_loads(&axle_positions, mass);

    // 1. Forward acceleration (+2 m/s^2) shifts weight to rear
    let accel_loads = susp.compute_dynamic_loads(&static_loads, &axle_positions, mass, 2.0);
    assert!(
        accel_loads[1] > static_loads[1],
        "Rear axle must gain weight on acceleration"
    );
    assert!(
        accel_loads[0] < static_loads[0],
        "Front axle must lose weight on acceleration"
    );

    // 2. Heavy braking (-5 m/s^2) shifts weight to front
    let brake_loads = susp.compute_dynamic_loads(&static_loads, &axle_positions, mass, -5.0);
    assert!(
        brake_loads[0] > static_loads[0],
        "Front axle must gain weight on braking"
    );
    assert!(
        brake_loads[1] < static_loads[1],
        "Rear axle must lose weight on braking"
    );
}
