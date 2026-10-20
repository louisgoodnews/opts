//! Unit tests of `vehicle_triggers.rs` (moved out of the file to keep it under the 500-line limit, refactoring cycle 1).

use super::*;

fn config() -> KeyboardConfig {
    KeyboardConfig::parse(
        b"[vehicles]\n[entry]\nhorn\n35\n1\n[entry]\nbus_doorfront0\n181\n0\n[entry]\nautomatic_D\n32\n0\n[entry]\nIBIS_1\n79\n4\n[entry]\nkw_s_2\n3\n4\n",
    )
}

#[test]
fn scan_codes_map_to_physical_keys() {
    assert_eq!(dik_to_keycode(25), Some(KeyCode::KeyP));
    assert_eq!(dik_to_keycode(44), Some(KeyCode::KeyZ));
    assert_eq!(dik_to_keycode(48), Some(KeyCode::KeyB));
    assert_eq!(dik_to_keycode(181), Some(KeyCode::NumpadDivide));
    assert_eq!(dik_to_keycode(2), Some(KeyCode::Digit1));
    assert_eq!(dik_to_keycode(11), Some(KeyCode::Digit0));
    assert_eq!(dik_to_keycode(68), Some(KeyCode::F10));
    assert_eq!(dik_to_keycode(500), None);
}

#[test]
fn reserved_keys_are_dropped_unless_ctrl_is_needed() {
    let b = ScriptKeyBindings::from_config(&config());
    let names: Vec<_> = b.0.iter().map(|b| b.trigger.as_str()).collect();
    // automatic_D sits on D (steering); kw_s_2 needs Ctrl and stays.
    assert_eq!(names, ["horn", "bus_doorfront0", "IBIS_1", "kw_s_2"]);
}

#[test]
fn press_and_release_raise_triggers() {
    let b = ScriptKeyBindings::from_config(&config());
    let mut keys = ButtonInput::<KeyCode>::default();
    keys.press(KeyCode::KeyH);
    keys.press(KeyCode::NumpadDivide);
    assert_eq!(b.fired(&keys), ["horn", "bus_doorfront0"]);
    keys.clear();
    keys.release(KeyCode::KeyH);
    keys.release(KeyCode::NumpadDivide);
    // Only the held binding reports its release.
    assert_eq!(b.fired(&keys), ["horn_off"]);
}

#[test]
fn chords_parse_and_print_canonically() {
    let c = KeyChord::parse("shift+ctrl+l").unwrap().unwrap();
    assert_eq!((c.key, c.shift, c.ctrl), (KeyCode::KeyL, true, true));
    assert_eq!(c.to_text().as_deref(), Some("Ctrl+Shift+L"));
    assert_eq!(KeyChord::parse("none").unwrap(), None);
    assert_eq!(KeyChord::parse("").unwrap(), None);
    let n = KeyChord::parse("NumpadDivide").unwrap().unwrap();
    assert_eq!(n.key, KeyCode::NumpadDivide);
    assert!(KeyChord::parse("Alt+L").is_err());
    assert!(KeyChord::parse("Banana").is_err());
    assert!(KeyChord::parse("W").unwrap().unwrap().is_reserved());
    assert!(!KeyChord::parse("Ctrl+W").unwrap().unwrap().is_reserved());
    assert!(!KeyChord::parse("E").unwrap().unwrap().is_reserved());
}

#[test]
fn compose_replaces_triggers_and_shadowed_chords() {
    let b = |t: &str, key, ctrl| ScriptKeyBinding {
        trigger: t.into(),
        key,
        shift: false,
        ctrl,
        held: false,
    };
    let base = [
        b("old_lights", KeyCode::KeyK, false),
        b("shadowed", KeyCode::KeyG, false),
        b("kept", KeyCode::KeyG, true),
    ];
    let own = vec![b("lights", KeyCode::KeyG, false)];
    let out = ScriptKeyBindings::compose(&base, &["old_lights"], own);
    let names: Vec<_> = out.0.iter().map(|b| b.trigger.as_str()).collect();
    assert_eq!(names, ["kept", "lights"]);
}

#[test]
fn modifiers_must_match_exactly() {
    let b = ScriptKeyBindings::from_config(&config());
    let mut keys = ButtonInput::<KeyCode>::default();
    keys.press(KeyCode::Numpad1);
    assert!(b.fired(&keys).is_empty());
    keys.clear();
    keys.release(KeyCode::Numpad1);
    keys.clear();
    keys.press(KeyCode::ControlLeft);
    keys.press(KeyCode::Numpad1);
    assert_eq!(b.fired(&keys), ["IBIS_1"]);
}

const KNOB: &str = "{trigger:knob}\n 1 (S.L.pressed)\n{end}\n\
{trigger:knob_drag}\n (L.S.mouse_y) -75 / (L.L.rot) + 0 max 1 min (S.L.rot) (L.L.drags) 1 + (S.L.drags)\n{end}\n\
{trigger:knob_off}\n 1 (S.L.released)\n{end}\n";

fn click_app() -> (App, Entity) {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .init_resource::<ScriptedClicks>()
        .add_systems(Update, scripted_click_system);
    let (engine, diags) = opts_sim::script::ScriptEngine::from_sources(&[("knob.osc", KNOB)]);
    assert!(
        diags
            .iter()
            .all(|d| d.severity != opts_sim::script::Severity::Error),
        "{diags:?}"
    );
    let e = app
        .world_mut()
        .spawn((PlayerVehicle, VehicleScripts::new(engine)))
        .id();
    (app, e)
}

fn var(app: &App, e: Entity, name: &str) -> f32 {
    app.world()
        .get::<VehicleScripts>(e)
        .and_then(|s| s.engine.lock().ok().map(|g| g.get(name)))
        .unwrap_or(f32::NAN)
}

fn frames(app: &mut App, n: u32) {
    for _ in 0..n {
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(50));
        app.update();
    }
}

#[test]
fn a_scripted_click_presses_drags_over_frames_and_releases() {
    let (mut app, e) = click_app();
    app.world_mut()
        .resource_mut::<ScriptedClicks>()
        .0
        .push(ScriptedClick::new("knob", Vec2::new(0.0, -60.0), 6, 0.0));
    frames(&mut app, 1);
    assert_eq!(var(&app, e, "pressed"), 1.0);
    assert_eq!(var(&app, e, "rot"), 0.0);
    frames(&mut app, 6);
    // The drag trigger ran once per frame: six calls, each adding 10 / 75.
    assert_eq!(var(&app, e, "drags"), 6.0);
    assert!(
        (var(&app, e, "rot") - 0.8).abs() < 1e-4,
        "rot {}",
        var(&app, e, "rot")
    );
    assert_eq!(var(&app, e, "released"), 0.0, "still held");
    frames(&mut app, 1);
    assert_eq!(var(&app, e, "released"), 1.0);
    assert!(app.world().resource::<ScriptedClicks>().0.is_empty());
    // The mouse values are back at 0 after the release.
    assert_eq!(var(&app, e, "mouse_y"), 0.0);
}

#[test]
fn a_click_without_a_drag_is_a_press_and_a_release() {
    let (mut app, e) = click_app();
    app.world_mut()
        .resource_mut::<ScriptedClicks>()
        .0
        .push(ScriptedClick::new("knob", Vec2::ZERO, 0, 0.0));
    frames(&mut app, 3);
    assert_eq!(
        (
            var(&app, e, "pressed"),
            var(&app, e, "released"),
            var(&app, e, "drags")
        ),
        (1.0, 1.0, 0.0)
    );
}

#[test]
fn the_tram_door_side_key_switches_between_right_and_left() {
    let src = "{trigger:kw_s_1}\n\t(L.L.Tuervorwahl_rechts_A) 0 =\n\t{if}\n\t\t1 (S.L.Tuervorwahl_rechts_A)\n\t{else}\n\t\t0 (S.L.Tuervorwahl_rechts_A)\n\t{endif}\n{end}\n{trigger:kw_s_2}\n\t(L.L.Tuervorwahl_links_A) 0 =\n\t{if}\n\t\t1 (S.L.Tuervorwahl_links_A)\n\t{else}\n\t\t0 (S.L.Tuervorwahl_links_A)\n\t{endif}\n{end}\n";
    let (mut engine, _) = opts_sim::script::ScriptEngine::from_sources(&[("t.osc", src)]);
    let sides = |e: &opts_sim::script::ScriptEngine| {
        (
            e.get("tuervorwahl_rechts_a") > 0.5,
            e.get("tuervorwahl_links_a") > 0.5,
        )
    };
    assert_eq!(sides(&engine), (false, false));
    assert!(toggle_door_side(&mut engine));
    assert_eq!(sides(&engine), (true, false), "none: the right side");
    toggle_door_side(&mut engine);
    assert_eq!(sides(&engine), (false, true), "then the left");
    toggle_door_side(&mut engine);
    assert_eq!(sides(&engine), (true, false), "and back");
}

#[test]
fn the_cabin_light_key_ends_on_or_off_whatever_the_switch_steps() {
    // The NLC's three-position switch: the toggle steps 0 -> 1 -> 2 -> 0.
    let src = "{trigger:cp_fahrerlicht_toggle}\n\t(L.L.cp_fahrerlicht_sw) 1 + (S.L.cp_fahrerlicht_sw) 2 >\n\t{if}\n\t\t0 (S.L.cp_fahrerlicht_sw)\n\t{endif}\n{end}\n{frame}\n{end}\n";
    let (mut engine, _) = opts_sim::script::ScriptEngine::from_sources(&[("t.osc", src)]);
    engine.set("cp_fahrerlicht_sw", 0.0);
    toggle_cabin_light(&mut engine);
    assert_eq!(engine.get("cp_fahrerlicht_sw"), 2.0, "off: stepped to on");
    toggle_cabin_light(&mut engine);
    assert_eq!(engine.get("cp_fahrerlicht_sw"), 0.0, "on: back to off");
    engine.set("cp_fahrerlicht_sw", 1.0);
    toggle_cabin_light(&mut engine);
    assert_eq!(engine.get("cp_fahrerlicht_sw"), 2.0, "auto: on");
    // A plain toggle (a tram): once.
    let src = "{trigger:cp_fahrerlicht_toggle}\n\t(L.L.LichtInnenFahrer_A_CP) 0 =\n\t{if}\n\t\t1 (S.L.LichtInnenFahrer_A_CP)\n\t{else}\n\t\t0 (S.L.LichtInnenFahrer_A_CP)\n\t{endif}\n{end}\n";
    let (mut tram, _) = opts_sim::script::ScriptEngine::from_sources(&[("t.osc", src)]);
    toggle_cabin_light(&mut tram);
    assert_eq!(tram.get("lichtinnenfahrer_a_cp"), 1.0);
    toggle_cabin_light(&mut tram);
    assert_eq!(tram.get("lichtinnenfahrer_a_cp"), 0.0);
}
