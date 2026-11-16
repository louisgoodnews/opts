//! Unit tests of the fuel and battery model (M-0123): consumption, multipliers, torque cut,
//! refuelling and the serialised form.

use super::*;

const HOUR: f32 = 3600.0;

#[test]
fn a_diesel_bus_at_full_throttle_burns_the_full_rate_for_an_hour() {
    let mut f = FuelState::full(FuelKind::Diesel, 250.0);
    f.step(1.0, HOUR);
    assert!((f.content - (250.0 - DIESEL_FULL_L_PER_H)).abs() < 1e-3);
}

#[test]
fn at_idle_the_diesel_burns_the_idle_rate() {
    let mut f = FuelState::full(FuelKind::Diesel, 250.0);
    f.step(0.0, HOUR);
    assert!((f.content - (250.0 - DIESEL_IDLE_L_PER_H)).abs() < 1e-3);
}

#[test]
fn half_throttle_burns_halfway_between_idle_and_full() {
    let f = FuelState::full(FuelKind::Diesel, 250.0);
    let half = (DIESEL_IDLE_L_PER_H + DIESEL_FULL_L_PER_H) / 2.0 / HOUR;
    assert!((f.rate_per_s(0.5) - half).abs() < 1e-7);
}

#[test]
fn accelerated_multiplies_the_rate_and_off_never_changes_the_level() {
    let mut acc = FuelState::full(FuelKind::Diesel, 250.0);
    acc.mode = FuelMode::Accelerated;
    acc.step(1.0, 60.0);
    let burnt = 250.0 - acc.content;
    assert!((burnt - DIESEL_FULL_L_PER_H * ACCELERATED_FACTOR / 60.0).abs() < 1e-3);

    let mut off = FuelState::full(FuelKind::Diesel, 250.0);
    off.mode = FuelMode::Off;
    off.step(1.0, HOUR);
    assert_eq!(off.content, 250.0);
    assert_eq!(FuelMode::Realistic.multiplier(), 1.0);
}

#[test]
fn an_empty_diesel_tank_cuts_the_torque_and_a_refill_restores_it() {
    let mut f = FuelState::full(FuelKind::Diesel, 100.0);
    f.set_level(0.0);
    assert!(f.is_empty());
    assert_eq!(f.torque_factor(), 0.0);
    f.refill();
    assert_eq!(f.torque_factor(), 1.0);
    assert_eq!(f.level(), 1.0);
}

#[test]
fn an_empty_battery_keeps_the_limp_mode_fraction() {
    let mut f = FuelState::full(FuelKind::Electric, 100.0);
    f.set_level(0.0);
    assert_eq!(f.torque_factor(), EMPTY_ELECTRIC_TORQUE_FRACTION);
}

#[test]
fn off_keeps_the_torque_even_when_the_store_reads_empty() {
    let mut f = FuelState::full(FuelKind::Diesel, 100.0);
    f.set_level(0.0);
    f.mode = FuelMode::Off;
    assert_eq!(f.torque_factor(), 1.0);
}

#[test]
fn the_low_level_warning_shows_at_the_threshold_and_below() {
    let mut f = FuelState::full(FuelKind::Diesel, 200.0);
    f.set_level(LOW_LEVEL + 0.01);
    assert!(!f.is_low());
    f.set_level(LOW_LEVEL);
    assert!(f.is_low());
}

#[test]
fn depletion_never_goes_below_empty_and_the_level_is_clamped() {
    let mut f = FuelState::full(FuelKind::Diesel, 10.0);
    f.step(1.0, 10.0 * HOUR);
    assert_eq!(f.content, 0.0);
    f.set_level(3.0);
    assert_eq!(f.level(), 1.0);
}

#[test]
fn the_state_survives_a_serialised_round_trip() {
    let mut f = FuelState::full(FuelKind::Electric, 120.0);
    f.mode = FuelMode::Accelerated;
    f.step(0.4, 17.0);
    let text = serde_json::to_string(&f).unwrap();
    let back: FuelState = serde_json::from_str(&text).unwrap();
    assert_eq!(back, f);
}

fn drive_for(sim: &mut crate::vehicle::VehicleSimulation, seconds: f32) {
    use crate::vehicle::inputs::GearPosition;
    sim.chassis.local_velocity.y = 5.0;
    sim.inputs.set_parking_brake(false);
    sim.inputs.set_throttle(1.0);
    sim.inputs.set_gear(GearPosition::Drive);
    for _ in 0..(seconds * 64.0) as usize {
        sim.step(1.0 / 64.0);
    }
}

#[test]
fn a_full_tank_keeps_the_drive_and_an_empty_one_loses_it() {
    let mut full = crate::vehicle::VehicleSimulation::new_standard_bus(11_500.0, 5.85);
    full.fuel = Some(FuelState::full(FuelKind::Diesel, 250.0));
    drive_for(&mut full, 2.0);

    let mut empty = crate::vehicle::VehicleSimulation::new_standard_bus(11_500.0, 5.85);
    let mut f = FuelState::full(FuelKind::Diesel, 250.0);
    f.set_level(0.0);
    empty.fuel = Some(f);
    drive_for(&mut empty, 2.0);
    assert!(full.chassis.forward_speed_m_s() > empty.chassis.forward_speed_m_s() + 0.2);
}

#[test]
fn a_vehicle_without_the_model_is_unchanged_by_the_fuel_step() {
    let mut sim = crate::vehicle::VehicleSimulation::new_standard_bus(11_500.0, 5.85);
    assert_eq!(sim.fuel_step(1.0 / 64.0), 1.0);
    assert!(sim.fuel.is_none());
}

#[test]
fn the_mode_keys_round_trip_and_unknown_text_is_realistic() {
    for mode in [FuelMode::Off, FuelMode::Realistic, FuelMode::Accelerated] {
        assert_eq!(FuelMode::parse(mode.key()), mode);
    }
    assert_eq!(FuelMode::parse(" OFF "), FuelMode::Off);
    assert_eq!(FuelMode::parse("bogus"), FuelMode::Realistic);
}

#[test]
fn ensure_fuel_keeps_an_existing_content_and_sets_the_mode() {
    let mut sim = crate::vehicle::VehicleSimulation::new_standard_bus(11_500.0, 5.85);
    sim.ensure_fuel(FuelKind::Diesel, 250.0, FuelMode::Accelerated);
    assert_eq!(sim.fuel_level(), Some(1.0));
    sim.fuel.as_mut().unwrap().set_level(0.5);
    sim.ensure_fuel(FuelKind::Electric, 10.0, FuelMode::Off);
    let f = sim.fuel.as_ref().unwrap();
    assert_eq!(f.kind, FuelKind::Diesel, "an existing model is not rebuilt");
    assert_eq!(f.mode, FuelMode::Off);
    assert!((f.level() - 0.5).abs() < 1e-6);
}

#[test]
fn refuel_restores_the_torque_of_an_empty_tank_and_reports_a_missing_model() {
    let mut sim = crate::vehicle::VehicleSimulation::new_standard_bus(11_500.0, 5.85);
    assert!(!sim.refuel());
    assert_eq!(sim.fuel_level(), None);
    sim.ensure_fuel(FuelKind::Diesel, 250.0, FuelMode::Realistic);
    sim.fuel.as_mut().unwrap().set_level(0.0);
    assert_eq!(sim.fuel_step(0.0), 0.0);
    assert!(sim.refuel());
    assert_eq!(sim.fuel_step(0.0), 1.0);
}
