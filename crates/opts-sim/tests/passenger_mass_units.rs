//! The people on board each unit of a consist are mass the consist moves (M-0124).

use opts_sim::vehicle::consist::{UnitAxle, UnitLoad};
use opts_sim::vehicle::dynamic_mass_kg;
use opts_sim::VehicleSimulation;

fn unit(mass_kg: f32) -> UnitLoad {
    UnitLoad {
        mass_kg,
        axles: vec![UnitAxle {
            longitudinal_m: -1.0,
            is_driven: false,
            rated_n: 100_000.0,
        }],
    }
}

fn train() -> VehicleSimulation {
    let mut sim = VehicleSimulation::new_standard_bus(10_000.0, 5.85);
    sim.set_trailers(vec![unit(8_000.0), unit(6_000.0)]);
    sim
}

#[test]
fn every_unit_adds_its_own_people_to_the_consist_mass() {
    let mut sim = train();
    assert_eq!(sim.consist_mass_kg(), 24_000.0);
    sim.set_unit_passenger_masses(&[
        dynamic_mass_kg(0.0, 10, 75.0),
        dynamic_mass_kg(0.0, 20, 75.0),
        dynamic_mass_kg(0.0, 4, 75.0),
    ]);
    assert_eq!(sim.passenger_mass_kg, 750.0);
    assert_eq!(sim.unit_passenger_mass_kg, [1_500.0, 300.0]);
    assert_eq!(sim.consist_mass_kg(), 24_000.0 + 2_550.0);
}

#[test]
fn a_short_list_leaves_the_other_units_empty_and_extra_entries_are_dropped() {
    let mut sim = train();
    sim.set_unit_passenger_masses(&[100.0]);
    assert_eq!(sim.unit_passenger_mass_kg, [0.0, 0.0]);
    sim.set_unit_passenger_masses(&[1.0, 2.0, 3.0, 4.0, 5.0]);
    assert_eq!(sim.unit_passenger_mass_kg, [2.0, 3.0]);
    assert_eq!(sim.consist_mass_kg(), 24_000.0 + 6.0);
}

#[test]
fn the_hitch_carries_the_people_behind_it() {
    let mut sim = train();
    let empty = sim.hitch_force_n(0).unwrap();
    sim.set_unit_passenger_masses(&[0.0, 1_000.0, 0.0]);
    // Joint 0 pulls both followers, joint 1 only the last one (no people there).
    let loaded = sim.hitch_force_n(0).unwrap();
    assert!(loaded >= empty);
    assert_eq!(sim.hitch_force_n(1).unwrap(), {
        let plain = train();
        plain.hitch_force_n(1).unwrap()
    });
}

#[test]
fn a_save_from_before_the_per_unit_load_loads_with_empty_units() {
    let mut sim = train();
    sim.set_unit_passenger_masses(&[750.0, 1_500.0, 300.0]);
    let mut value = serde_json::to_value(&sim).expect("serialises");
    value
        .as_object_mut()
        .expect("an object")
        .remove("unit_passenger_mass_kg");
    let old: VehicleSimulation = serde_json::from_value(value).expect("old save loads");
    assert!(old.unit_passenger_mass_kg.is_empty());
    assert_eq!(old.passenger_mass_kg, 750.0);
    assert_eq!(old.consist_mass_kg(), 24_000.0 + 750.0);
    // And a current one round-trips.
    let again: VehicleSimulation =
        serde_json::from_value(serde_json::to_value(&sim).unwrap()).unwrap();
    assert_eq!(again.unit_passenger_mass_kg, [1_500.0, 300.0]);
}

#[test]
fn a_single_unit_is_unchanged() {
    let mut sim = VehicleSimulation::new_standard_bus(11_000.0, 5.85);
    sim.set_unit_passenger_masses(&[4_500.0]);
    assert_eq!(sim.passenger_mass_kg, 4_500.0);
    assert!(sim.unit_passenger_mass_kg.is_empty());
    assert_eq!(sim.consist_mass_kg(), 15_500.0);
}
