//! Tests of the start procedures on small stand-in scripts (the real vehicles' procedures are
//! checked by `crates/opts-game/tests/vehicle_start.rs`).

use super::*;

/// A MAN-like vehicle: the battery key puts the busbar up, the start key (held) starts.
const MAN_LIKE: &str = "\
{init}
    120 (S.L.engine_tank_content)
{end}
{trigger:cp_batterietrennschalter_toggle}
    1 (S.L.elec_busbar_main)
{end}
{trigger:kw_m_enginestart}
    (L.L.elec_busbar_main) 0.3 >
    {if}
        1 (S.L.engine_starter)
    {endif}
{end}
{trigger:kw_m_enginestart_off}
    (L.L.engine_starter)
    {if}
        1 (S.L.engine_on)
    {endif}
    0 (S.L.engine_starter)
{end}
{trigger:kw_scheinwerfer_toggle}
    1 (S.L.lights_abbl)
{end}
";

/// A stepped-key vehicle: owns `cp_keyboard_e_m`, which counts the battery key's presses.
const STEPPED: &str = "\
{trigger:cp_batterietrennschalter_toggle}
    (L.L.cp_keyboard_e_m) 1 + (S.L.cp_keyboard_e_m)
    (L.L.cp_keyboard_e_m) 1 =
    {if}
        1 (S.L.elec_busbar_main)
    {endif}
    (L.L.cp_keyboard_e_m) 3 =
    {if}
        1 (S.L.engine_on)
    {endif}
{end}
";

fn engine(src: &str) -> ScriptEngine {
    let (mut e, diags) = ScriptEngine::from_sources(&[("t.osc", src)]);
    assert!(diags.is_empty(), "{diags:?}");
    e.run_init();
    e
}

/// Runs the recipe for `seconds` of 60 Hz frames.
fn drive(e: &mut ScriptEngine, run: &mut SetupRun, setup: &VehicleSetup, seconds: f32) -> bool {
    let mut settled = false;
    let mut now = 0.0;
    while now < seconds {
        now += 1.0 / 60.0;
        e.run_frame(1.0 / 60.0, false);
        settled = run.advance(e, setup, now);
    }
    settled
}

fn at(degree: f32) -> VehicleSetup {
    VehicleSetup {
        degree,
        ..VehicleSetup::default()
    }
}

#[test]
fn degree_zero_starts_nothing() {
    let mut e = engine(MAN_LIKE);
    let mut run = SetupRun::default();
    assert!(
        drive(&mut e, &mut run, &at(0.0), 10.0),
        "nothing to do is settled"
    );
    assert_eq!(e.get("elec_busbar_main"), 0.0);
    assert_eq!(e.get("engine_on"), 0.0);
}

#[test]
fn each_degree_stops_at_its_state() {
    for (degree, electrics, engine_on) in [
        (0.25, 1.0, 0.0),
        (0.5, 1.0, 0.0),
        (0.75, 1.0, 1.0),
        (1.0, 1.0, 1.0),
    ] {
        let mut e = engine(MAN_LIKE);
        let mut run = SetupRun::default();
        assert!(
            drive(&mut e, &mut run, &at(degree), 12.0),
            "degree {degree} settles"
        );
        assert_eq!(
            e.get("elec_busbar_main"),
            electrics,
            "electrics at {degree}"
        );
        assert_eq!(e.get("engine_on"), engine_on, "engine at {degree}");
    }
}

#[test]
fn raising_the_degree_later_continues_from_where_it_stopped() {
    let mut e = engine(MAN_LIKE);
    let mut run = SetupRun::default();
    drive(&mut e, &mut run, &at(0.25), 6.0);
    assert_eq!(e.get("engine_on"), 0.0);
    // Time continues: the run is driven with a larger `now` than before.
    let mut now = 6.0;
    let want = at(0.75);
    while now < 20.0 {
        now += 1.0 / 60.0;
        e.run_frame(1.0 / 60.0, false);
        run.advance(&mut e, &want, now);
    }
    assert_eq!(e.get("engine_on"), 1.0);
}

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

/// A tram-like vehicle: scripts without an engine or a busbar.
const TRAM_LIKE: &str = "\
{trigger:kw_s_1}
    1 (S.L.traction)
{end}
";

#[test]
fn a_vehicle_without_an_engine_settles_at_once_and_counts_as_started() {
    let mut e = engine(TRAM_LIKE);
    let mut run = SetupRun::default();
    assert!(
        drive(&mut e, &mut run, &at(1.0), 6.0),
        "nothing to start, nothing to wait for"
    );
    assert!(!has_engine(&e));
    assert!(has_engine(&engine(MAN_LIKE)));
}

/// A key turned with the mouse, in a part that also has the `engine_start_legacy` constant:
/// electrics from 0.2, the starter past 0.9, springs back to a third on release.
const MOUSE_KEY: &str = "\
{trigger:cp_key_rot_drag}
    (L.S.mouse_y) -30 / (L.L.cp_key_rot) + 0 max 1 min (S.L.cp_key_rot) s1
    l1 0.2 >
    {if}
        1 (S.L.elec_busbar_main)
    {endif}
    l1 0.9 > (L.L.elec_busbar_main) &&
    {if}
        1 (S.L.engine_on)
    {endif}
{end}
{trigger:cp_key_rot_off}
    (L.L.cp_key_rot) 0.33 min (S.L.cp_key_rot)
{end}
";

fn mouse_key() -> ScriptEngine {
    let mut e = engine(MOUSE_KEY);
    e.consts_mut().set_scalar("engine_start_legacy", 0.0);
    e
}

#[test]
fn a_mouse_turned_key_stops_at_the_electrics_and_springs_back() {
    let mut e = mouse_key();
    assert_eq!(recipe_for(&e).name, "ignition key turned by mouse");
    drive(&mut e, &mut SetupRun::default(), &at(ELECTRICS), 4.0);
    assert_eq!(e.get("elec_busbar_main"), 1.0, "electrics on");
    assert_eq!(
        e.get("engine_on"),
        0.0,
        "not started at the electrics degree"
    );
    assert!(
        e.get("cp_key_rot") < 0.4,
        "key sprang back: {}",
        e.get("cp_key_rot")
    );
}

#[test]
fn a_mouse_turned_key_is_dragged_past_the_starter_position() {
    let mut e = mouse_key();
    drive(&mut e, &mut SetupRun::default(), &at(ENGINE), 8.0);
    assert_eq!(e.get("engine_on"), 1.0, "started past 0.9");
}

#[test]
fn recipes_recognise_a_trigger_and_a_constant() {
    let valiant = engine("{trigger:cp_start_key_rot_drag}\n    1 (S.L.x)\n{end}\n");
    assert_eq!(recipe_for(&valiant).name, "three-step key (battery button)");
    let mut legacy = engine(MOUSE_KEY);
    legacy.consts_mut().set_scalar("engine_start_legacy", 1.0);
    assert_eq!(
        recipe_for(&legacy).name,
        "battery switch and start key",
        "the legacy C2 takes the battery switch and start key"
    );
}
