//! People on board take steering authority away (M-0124): a loaded bus turns into a steered
//! path more slowly than an empty one, the steady turn is unchanged, and with no people the
//! response is the empty bus's law exactly.

use opts_sim::vehicle::dynamic_mass_kg;
use opts_sim::VehicleSimulation;

const DT: f32 = 1.0 / 64.0;
const CURB_KG: f32 = 11_000.0;
const SPEED_M_S: f32 = 5.0;
const STEER_LOCK_RAD: f32 = 0.82;

fn bus(people: usize) -> VehicleSimulation {
    let mut sim = VehicleSimulation::new_standard_bus(CURB_KG, 5.85);
    sim.passenger_mass_kg = dynamic_mass_kg(0.0, people, 75.0);
    sim
}

/// Yaw rate after `seconds` of a turn at full steering lock and a held speed, as a share of the
/// steady rate the steering asks for. The speed is held so the target does not move.
fn turn_share(people: usize, seconds: f32) -> f32 {
    let mut sim = bus(people);
    let target = (SPEED_M_S / sim.wheelbase_m) * STEER_LOCK_RAD.tan();
    sim.inputs.set_steering(1.0);
    for _ in 0..(seconds / DT) as usize {
        sim.chassis.local_velocity.y = SPEED_M_S;
        sim.step(DT);
    }
    sim.chassis.yaw_rate_rad_s / target
}

#[test]
fn an_empty_bus_turns_with_the_empty_response_law() {
    // The chassis yaw lag is an Euler step of rate += 10 dt (target - rate) per step, so after
    // n steps the share is 1 - (1 - 10 dt)^n for the empty bus.
    let t = 0.3;
    let steps = (t / DT) as i32;
    let expected = 1.0 - (1.0 - 10.0 * DT).powi(steps);
    let share = turn_share(0, t);
    assert!(
        (share - expected).abs() < 1e-3,
        "empty share {share:.4} vs law {expected:.4}"
    );
}

#[test]
fn a_loaded_bus_responds_less_at_higher_load() {
    let loads = [0, 20, 40, 60, 80, 120];
    let shares: Vec<f32> = loads.iter().map(|&p| turn_share(p, 0.3)).collect();
    for pair in shares.windows(2) {
        assert!(
            pair[1] < pair[0],
            "response must fall with load: {shares:?}"
        );
    }
}

#[test]
fn a_loaded_bus_reaches_the_same_steady_turn() {
    for people in [0, 80] {
        let share = turn_share(people, 3.0);
        assert!(share > 0.999, "{people} people: steady share {share:.5}");
    }
}
