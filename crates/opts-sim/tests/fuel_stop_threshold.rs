//! The refuel speed threshold is pinned at 0.5 m/s, either direction (M-0123): just below it
//! refuelling is allowed, at and just above it is refused.

use opts_sim::vehicle::fuel::{FuelKind, FuelMode};
use opts_sim::VehicleSimulation;

fn empty_bus() -> VehicleSimulation {
    let mut sim = VehicleSimulation::new_standard_bus(11_500.0, 5.85);
    sim.ensure_fuel(FuelKind::Diesel, 250.0, FuelMode::Realistic);
    sim.fuel.as_mut().unwrap().set_level(0.0);
    sim
}

/// Refuels at the given signed speed (forward positive) and returns the answer and the level.
fn refuel_at(speed_m_s: f32) -> (bool, Option<f32>) {
    let mut sim = empty_bus();
    sim.chassis.local_velocity.y = speed_m_s;
    let answer = sim.refuel();
    (answer, sim.fuel_level())
}

#[test]
fn just_below_the_threshold_allows_refuel_forward_and_reverse() {
    for speed in [0.49_f32, -0.49] {
        let (answer, level) = refuel_at(speed);
        assert!(answer, "{speed} m/s is below 0.5 m/s, refuel allowed");
        assert_eq!(level, Some(1.0), "{speed} m/s fills the store");
    }
}

#[test]
fn just_above_the_threshold_refuses_refuel_forward_and_reverse() {
    for speed in [0.51_f32, -0.51] {
        let (answer, level) = refuel_at(speed);
        assert!(!answer, "{speed} m/s is above 0.5 m/s, refuel refused");
        assert_eq!(level, Some(0.0), "{speed} m/s leaves the store unchanged");
    }
}

#[test]
fn exactly_the_threshold_counts_as_moving() {
    for speed in [0.5_f32, -0.5] {
        let (answer, level) = refuel_at(speed);
        assert!(!answer, "{speed} m/s is at the threshold, refused");
        assert_eq!(level, Some(0.0));
    }
}
