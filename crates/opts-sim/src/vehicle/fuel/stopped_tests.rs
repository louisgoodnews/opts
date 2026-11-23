//! Refuelling is refused while the vehicle moves and allowed when it is stopped (M-0123).

use super::*;
use crate::vehicle::VehicleSimulation;

fn empty_bus() -> VehicleSimulation {
    let mut sim = VehicleSimulation::new_standard_bus(11_500.0, 5.85);
    sim.ensure_fuel(FuelKind::Diesel, 250.0, FuelMode::Realistic);
    sim.fuel.as_mut().unwrap().set_level(0.0);
    sim
}

#[test]
fn refuel_is_refused_while_the_vehicle_moves() {
    let mut sim = empty_bus();
    sim.chassis.local_velocity.y = 5.0;
    assert!(!sim.refuel());
    assert_eq!(
        sim.fuel_level(),
        Some(0.0),
        "a refused refuel leaves the store unchanged"
    );
    sim.chassis.local_velocity.y = -5.0;
    assert!(!sim.refuel(), "reversing counts as moving");
    assert_eq!(sim.fuel_level(), Some(0.0));
}

#[test]
fn refuel_is_allowed_when_stopped() {
    let mut sim = empty_bus();
    sim.chassis.local_velocity.y = 0.0;
    assert!(sim.refuel());
    assert_eq!(sim.fuel_level(), Some(1.0));
}
