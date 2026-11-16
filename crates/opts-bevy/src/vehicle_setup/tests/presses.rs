//! The key presses of a start: the stepped key, the tank and the lights.

use super::*;

#[test]
fn the_recipe_follows_the_family_of_the_scripts() {
    assert_eq!(
        recipe_for(&engine(MAN_LIKE)).name,
        "battery switch and start key"
    );
    assert_eq!(recipe_for(&engine(STEPPED)).name, "stepped ignition key");
}

#[test]
fn a_stepped_key_takes_three_presses_to_the_engine() {
    let mut e = engine(STEPPED);
    let mut run = SetupRun::default();
    drive(&mut e, &mut run, &at(0.5), 10.0);
    assert_eq!(
        e.get("cp_keyboard_e_m"),
        2.0,
        "electrics, then drive position"
    );
    assert_eq!(e.get("engine_on"), 0.0);
    let mut e = engine(STEPPED);
    let mut run = SetupRun::default();
    drive(&mut e, &mut run, &at(0.75), 12.0);
    assert_eq!(e.get("cp_keyboard_e_m"), 3.0);
    assert_eq!(e.get("engine_on"), 1.0);
}

#[test]
fn the_tank_is_filled_to_the_players_share_once() {
    let mut e = engine(MAN_LIKE);
    let mut run = SetupRun::default();
    let setup = VehicleSetup {
        fuel: 0.4,
        ..at(0.0)
    };
    drive(&mut e, &mut run, &setup, 2.0);
    assert!(
        (e.get("engine_tank_content") - 100.0).abs() < 0.01,
        "{}",
        e.get("engine_tank_content")
    );
    e.set("engine_tank_content", 77.0);
    drive(&mut e, &mut run, &setup, 2.0);
    assert_eq!(e.get("engine_tank_content"), 77.0, "not refilled");
}

#[test]
fn the_lights_come_on_with_the_electrics() {
    let mut e = engine(MAN_LIKE);
    let mut run = SetupRun::default();
    let setup = VehicleSetup {
        lights: StartLights::Dipped,
        ..at(0.25)
    };
    drive(&mut e, &mut run, &setup, 8.0);
    assert_eq!(e.get("lights_abbl"), 1.0);
    let mut e = engine(MAN_LIKE);
    let mut run = SetupRun::default();
    drive(&mut e, &mut run, &at(0.25), 8.0);
    assert_eq!(e.get("lights_abbl"), 0.0, "off by default");
}

#[test]
fn stored_lights_text_is_parsed() {
    assert_eq!(StartLights::parse("dipped"), StartLights::Dipped);
    assert_eq!(StartLights::parse("parking"), StartLights::Parking);
    assert_eq!(StartLights::parse("nonsense"), StartLights::Off);
}
