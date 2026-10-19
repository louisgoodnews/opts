//! Script triggers and the vehicle keys.

use super::*;

fn variable(app: &mut App, name: &str) -> Option<f32> {
    let mut q = app
        .world_mut()
        .query_filtered::<&opts_bevy::VehicleScripts, With<PlayerVehicle>>();
    let scripts = q.iter(app.world()).next()?;
    let engine = scripts.engine.lock().ok()?;
    engine
        .variables()
        .into_iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(name))
        .map(|(_, v)| v)
}

const SCRIPT: &str = "\
{trigger:horn}
    1 (S.L.hornv)
{end}

{trigger:horn_off}
    0 (S.L.hornv)
{end}

{trigger:bus_doorfront0}
    1 (S.L.doorv)
{end}
";

#[test]
fn triggers_raise_script_events_with_aliases_holds_and_gates() {
    let mut app = drive_app();
    let keyboard = opts_core::formats::engine_config::KeyboardConfig::parse(
        b"[vehicles]\n[entry]\nbus_doorfront0\n181\n0\n[entry]\nhorn\n35\n1\n",
    );
    app.insert_resource(ScriptKeyBindings::from_config(&keyboard));
    let (mut engine, _) = opts_sim::script::ScriptEngine::from_sources(&[("t.osc", SCRIPT)]);
    engine.run_init();
    let vehicle = {
        let mut q = app
            .world_mut()
            .query_filtered::<Entity, With<PlayerVehicle>>();
        q.iter(app.world()).next().expect("vehicle")
    };
    app.world_mut()
        .entity_mut(vehicle)
        .insert(opts_bevy::VehicleScripts::new(engine));
    // A held binding (the horn): `horn` now, `horn_off` after the hold, in simulation time.
    let horn = call(
        &mut app,
        "drive.trigger",
        json!({"name": "horn", "hold_ms": 100}),
    )
    .expect("horn");
    assert_eq!(horn["handled"], true);
    assert_eq!(horn["held_ms"], 100);
    assert_eq!(variable(&mut app, "hornv"), Some(1.0));
    for _ in 0..10 {
        app.update();
    }
    assert_eq!(
        variable(&mut app, "hornv"),
        Some(0.0),
        "horn_off followed the hold"
    );
    // A short name resolves to the vehicle's own trigger; `drive.set` takes trigger names too.
    let door = call(
        &mut app,
        "drive.set",
        json!({"control": "door_front", "value": true}),
    )
    .expect("door");
    assert_eq!(door["name"], "bus_doorfront0");
    assert_eq!(variable(&mut app, "doorv"), Some(1.0));
    // A name no script handles is raised and reported as not handled.
    let none = call(
        &mut app,
        "drive.trigger",
        json!({"name": "no_such_trigger"}),
    )
    .expect("raise");
    assert_eq!(none["handled"], false);
    // The modal gate blocks triggers like a key press.
    app.insert_resource(ModalActive(true));
    let gated = call(&mut app, "drive.trigger", json!({"name": "horn"})).expect_err("gated");
    assert_eq!(gated.code, code::GATED);
    // The listing shows the bound triggers with their aliases.
    app.insert_resource(ModalActive(false));
    let listing = call(&mut app, "drive.controls", json!({})).expect("controls");
    let triggers = listing["triggers"].as_array().expect("triggers");
    assert!(triggers
        .iter()
        .any(|t| t["name"] == "bus_doorfront0" && t["alias"] == "door_front"));
    assert!(triggers
        .iter()
        .any(|t| t["name"] == "horn" && t["held"] == true));
}

const KEY_SCRIPT: &str = "\
{trigger:kw_scheinwerfer_toggle}
    1 (S.L.lightsv)
{end}

{trigger:kw_m_enginestart}
    1 (S.L.startv)
{end}

{trigger:kw_m_enginestart_off}
    0 (S.L.startv)
{end}
";

#[test]
fn vehicle_keys_press_actions_by_name_or_key_and_list_the_bindings() {
    let mut app = drive_app();
    app.insert_resource(opts_game::settings_registry::SettingsRegistry::with_builtin());
    app.insert_resource(opts_game::settings::GameSettings::default());
    app.add_plugins(opts_game::vehicle_keys::VehicleKeysPlugin);
    app.update();
    let (mut engine, _) = opts_sim::script::ScriptEngine::from_sources(&[("k.osc", KEY_SCRIPT)]);
    engine.run_init();
    let vehicle = {
        let mut q = app
            .world_mut()
            .query_filtered::<Entity, With<PlayerVehicle>>();
        q.iter(app.world()).next().expect("vehicle")
    };
    app.world_mut()
        .entity_mut(vehicle)
        .insert(opts_bevy::VehicleScripts::new(engine));
    // By action: whatever key it has.
    let r = call(&mut app, "vehicle.key", json!({"action": "lights"})).expect("lights");
    assert_eq!(r["raised"][0]["raised"], "kw_scheinwerfer_toggle");
    assert_eq!(r["raised"][0]["handled"], true);
    assert_eq!(variable(&mut app, "lightsv"), Some(1.0));
    // By key: the default L raises the same trigger; an unbound key is an error.
    let r = call(&mut app, "vehicle.key", json!({"key": "L"})).expect("key L");
    assert_eq!(r["raised"][0]["action"], "lights");
    let e = call(&mut app, "vehicle.key", json!({"key": "F9"})).expect_err("unbound");
    assert_eq!(e.code, code::INVALID_PARAMS);
    let e = call(&mut app, "vehicle.key", json!({"action": "warp_drive"})).expect_err("unknown");
    assert_eq!(e.code, code::INVALID_PARAMS);
    let e = call(&mut app, "vehicle.key", json!({})).expect_err("nothing");
    assert_eq!(e.code, code::INVALID_PARAMS);
    // Engine start is held: `_off` follows after the hold.
    let r = call(
        &mut app,
        "vehicle.key",
        json!({"action": "engine_start", "hold_ms": 100}),
    )
    .expect("start");
    assert_eq!(r["raised"][0]["held_ms"], 100);
    assert_eq!(variable(&mut app, "startv"), Some(1.0));
    for _ in 0..10 {
        app.update();
    }
    assert_eq!(variable(&mut app, "startv"), Some(0.0));
    // The listing names the actions with their keys and defaults.
    let l = call(&mut app, "vehicle.keys", json!({})).expect("keys");
    let a = l["actions"].as_array().expect("actions");
    assert!(a
        .iter()
        .any(|x| x["action"] == "engine_start" && x["key"] == "M" && x["default"] == "M"));
    // The modal gate blocks it like a key press.
    app.insert_resource(ModalActive(true));
    let e = call(&mut app, "vehicle.key", json!({"action": "lights"})).expect_err("gated");
    assert_eq!(e.code, code::GATED);
}
