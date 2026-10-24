//! People on board are mass the vehicle moves (M-0124): a full bus accelerates slower and stops
//! later than an empty one, and a count of 0 is the curb mass exactly.

use opts_sim::vehicle::dynamic_mass_kg;
use opts_sim::vehicle::inputs::GearPosition;
use opts_sim::VehicleSimulation;

const DT: f32 = 1.0 / 64.0;
const CURB_KG: f32 = 11_000.0;

fn bus(people: usize) -> VehicleSimulation {
    let mut sim = VehicleSimulation::new_standard_bus(CURB_KG, 5.85);
    sim.passenger_mass_kg = dynamic_mass_kg(0.0, people, 75.0);
    sim
}

fn full_throttle(sim: &mut VehicleSimulation, seconds: f32) -> f32 {
    sim.inputs.set_gear(GearPosition::Drive);
    sim.inputs.set_parking_brake(false);
    sim.inputs.set_throttle(1.0);
    for _ in 0..(seconds / DT) as usize {
        sim.step(DT);
    }
    sim.chassis.forward_speed_m_s()
}

#[test]
fn the_dynamic_mass_adds_the_people_and_ignores_a_negative_average() {
    assert_eq!(dynamic_mass_kg(11_000.0, 60, 75.0), 15_500.0);
    assert_eq!(dynamic_mass_kg(11_000.0, 0, 75.0), 11_000.0);
    assert_eq!(dynamic_mass_kg(11_000.0, 60, -5.0), 11_000.0);
}

#[test]
fn a_bus_without_people_moves_exactly_its_curb_mass() {
    assert_eq!(bus(0).consist_mass_kg(), CURB_KG);
    assert_eq!(bus(60).consist_mass_kg(), 15_500.0);
}

#[test]
fn a_full_bus_accelerates_slower_than_an_empty_one() {
    let (mut empty, mut full) = (bus(0), bus(80));
    let (v_empty, v_full) = (
        full_throttle(&mut empty, 6.0),
        full_throttle(&mut full, 6.0),
    );
    // Same force on 17 t instead of 11 t: the speed gain is bounded by the mass ratio.
    assert!(
        v_full < v_empty * 0.92,
        "full {v_full:.2} m/s vs empty {v_empty:.2} m/s"
    );
}

#[test]
fn a_full_bus_needs_a_longer_braking_distance() {
    let stop = |people: usize| {
        let mut sim = bus(people);
        sim.inputs.set_gear(GearPosition::Drive);
        sim.inputs.set_parking_brake(false);
        sim.chassis.local_velocity.y = 50.0 / 3.6;
        sim.inputs.set_throttle(0.0);
        sim.inputs.set_service_brake(1.0);
        let start = sim.chassis.position;
        for _ in 0..(20.0 / DT) as usize {
            sim.step(DT);
        }
        (sim.chassis.position - start).length()
    };
    let (empty, full) = (stop(0), stop(80));
    assert!(full > empty, "full {full:.1} m vs empty {empty:.1} m");
}

fn settle(sim: &mut VehicleSimulation, seconds: f32) {
    sim.inputs.set_parking_brake(true);
    for _ in 0..(seconds / DT) as usize {
        sim.step(DT);
    }
}

#[test]
fn people_on_board_sink_the_body_by_their_weight_over_the_spring_rate() {
    let (mut empty, mut full) = (bus(0), bus(80));
    settle(&mut empty, 6.0);
    settle(&mut full, 6.0);
    let (e, f) = (empty.body.as_ref().unwrap(), full.body.as_ref().unwrap());
    let weight = 80.0 * 75.0 * 9.81;
    let stiffness: f32 = f
        .wheels
        .iter()
        .map(|w| w.spring_n_m * w.spring_factor)
        .sum();
    let expected = weight / stiffness;
    let sunk = e.heave_m - f.heave_m;
    assert!(
        (sunk - expected).abs() <= 0.15 * expected,
        "sank {sunk:.4} m, expected {expected:.4} m"
    );
}

#[test]
fn the_wheels_carry_the_curb_weight_plus_the_people() {
    let mut full = bus(80);
    settle(&mut full, 6.0);
    let body = full.body.as_ref().unwrap();
    let carried = body.total_load_n();
    let expected = (CURB_KG + 80.0 * 75.0) * 9.81;
    assert!(
        (carried - expected).abs() <= 0.02 * expected,
        "wheels carry {carried:.0} N, expected {expected:.0} N"
    );
}

#[test]
fn the_extra_weight_loads_the_axles_by_their_lever_arms() {
    let (mut empty, mut full) = (bus(0), bus(80));
    settle(&mut empty, 6.0);
    settle(&mut full, 6.0);
    let (e, f) = (empty.body.as_ref().unwrap(), full.body.as_ref().unwrap());
    let axles = empty.axles.len();
    let gains: Vec<f32> = (0..axles)
        .map(|i| f.axle_load_n(i) - e.axle_load_n(i))
        .collect();
    assert!(
        gains.iter().all(|g| *g > 0.0),
        "every axle gains load: {gains:?}"
    );
    let total: f32 = gains.iter().sum();
    assert!((total - 80.0 * 75.0 * 9.81).abs() < 0.03 * total, "{total}");
}
