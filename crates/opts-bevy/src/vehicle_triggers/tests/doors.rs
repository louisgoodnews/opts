//! Door-side and cabin-light toggle tests.

use super::*;

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
