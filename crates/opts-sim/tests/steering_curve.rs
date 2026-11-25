//! The steering curve (ISS-057): a small wheel input steers less than the linear law did, and
//! the full lock keeps the steady turn of the linear law. Before the curve, a 0.2 input turned
//! the bus as a 0.164 rad central angle (about 9.4 degrees) does; now it turns as 0.0853 rad.

use opts_sim::VehicleSimulation;

const DT: f32 = 1.0 / 64.0;
const CURB_KG: f32 = 11_000.0;
const SPEED_M_S: f32 = 5.0;
const LOCK_RAD: f32 = 0.82;

/// The curve of `opts_sim` (`f(x) = 0.5 x + 0.5 x^3`), replicated here for the expectation.
fn curved(x: f32) -> f32 {
    0.5 * x + 0.5 * x * x * x
}

/// Steady yaw rate after `seconds` at a held speed and `input`, as a share of the yaw rate the
/// curved central angle asks for.
fn steady_share(input: f32, seconds: f32) -> f32 {
    let mut sim = VehicleSimulation::new_standard_bus(CURB_KG, 5.85);
    let target = (SPEED_M_S / sim.wheelbase_m) * (LOCK_RAD * curved(input)).tan();
    sim.inputs.set_steering(input);
    for _ in 0..(seconds / DT) as usize {
        sim.chassis.local_velocity.y = SPEED_M_S;
        sim.step(DT);
    }
    sim.chassis.yaw_rate_rad_s / target
}

#[test]
fn a_small_input_turns_by_the_curved_central_angle() {
    let share = steady_share(0.2, 3.0);
    assert!(
        (share - 1.0).abs() < 0.005,
        "0.2 input: steady share {share:.4} of the curved turn"
    );
}

#[test]
fn a_medium_input_turns_by_the_curved_central_angle() {
    let share = steady_share(0.6, 3.0);
    assert!(
        (share - 1.0).abs() < 0.005,
        "0.6 input: steady share {share:.4} of the curved turn"
    );
}

#[test]
fn full_lock_turns_by_the_unchanged_linear_law() {
    let share = steady_share(1.0, 3.0);
    assert!((share - 1.0).abs() < 0.005, "full lock share {share:.4}");
}
