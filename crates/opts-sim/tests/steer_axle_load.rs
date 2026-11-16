//! The steering authority reads the load on the steered axle (M-0124 steer-axle input): the
//! steered axle's share of the curb mass and of the people on board come from the axle split of
//! the suspension model, with the people at the middle of the axle span.

use opts_sim::vehicle::dynamic_mass_kg;
use opts_sim::VehicleSimulation;

const DT: f32 = 1.0 / 64.0;
const CURB_KG: f32 = 11_000.0;
const SPEED_M_S: f32 = 5.0;
const STEER_LOCK_RAD: f32 = 0.82;
const G: f32 = 9.81;

fn bus(people: usize, curb_cg_m: f32) -> VehicleSimulation {
    let mut sim = VehicleSimulation::new_standard_bus(CURB_KG, 5.85);
    sim.passenger_mass_kg = dynamic_mass_kg(0.0, people, 75.0);
    sim.suspension.cg_longitudinal_m = curb_cg_m;
    sim
}

/// Yaw rate share of the steady turn after `seconds` at full lock and a held speed.
fn turn_share(people: usize, curb_cg_m: f32, seconds: f32) -> f32 {
    let mut sim = bus(people, curb_cg_m);
    let target = (SPEED_M_S / sim.wheelbase_m) * STEER_LOCK_RAD.tan();
    sim.inputs.set_steering(1.0);
    for _ in 0..(seconds / DT) as usize {
        sim.chassis.local_velocity.y = SPEED_M_S;
        sim.step(DT);
    }
    sim.chassis.yaw_rate_rad_s / target
}

#[test]
fn the_setting_off_leaves_the_steering_bit_for_bit_unchanged() {
    // The setting off means no people in the mass: the same run as a bus never given a count.
    let mut off = VehicleSimulation::new_standard_bus(CURB_KG, 5.85);
    let mut zero = bus(0, 0.0);
    off.inputs.set_steering(1.0);
    zero.inputs.set_steering(1.0);
    for _ in 0..200 {
        for sim in [&mut off, &mut zero] {
            sim.chassis.local_velocity.y = SPEED_M_S;
            sim.step(DT);
        }
    }
    assert_eq!(
        off.chassis.yaw_rate_rad_s.to_bits(),
        zero.chassis.yaw_rate_rad_s.to_bits()
    );
    let (unladen, people) = zero.steer_axle_weights_n();
    assert_eq!(people, 0.0);
    assert!(unladen > 0.0);
}

#[test]
fn the_steered_axle_load_from_people_rises_with_the_count() {
    let mut last = -1.0_f32;
    for people in [0, 20, 40, 60, 80, 120] {
        let sim = bus(people, 0.0);
        let (unladen, on_steer) = sim.steer_axle_weights_n();
        assert!(
            on_steer > last,
            "{people} people: {on_steer} not above {last}"
        );
        assert!(unladen > 0.0);
        last = on_steer;
    }
}

#[test]
fn the_curb_split_at_its_own_centre_of_gravity_sets_the_steered_share() {
    // A curb centre of gravity forward of the middle loads the steered (front) axle more than
    // the people do, who always sit at the middle of the span.
    let sim = bus(60, 2.0);
    let (unladen, on_steer) = sim.steer_axle_weights_n();
    let curb_weight = sim.chassis.total_mass_kg * G;
    assert!(
        unladen / curb_weight > 0.5,
        "curb share {}",
        unladen / curb_weight
    );
    let people_weight = sim.passenger_mass_kg * G;
    assert!((on_steer / people_weight - 0.5).abs() < 1e-4);
}

#[test]
fn a_curb_centre_forward_of_the_middle_responds_more_with_load() {
    // The steering authority is unladen / (unladen + load) on the steered axle, so a heavier
    // front (more curb weight there) keeps more of the yaw response under the same people.
    let middle = turn_share(80, 0.0, 0.3);
    let forward = turn_share(80, 2.0, 0.3);
    assert!(
        forward > middle,
        "forward {forward:.4} vs middle {middle:.4}"
    );
}
