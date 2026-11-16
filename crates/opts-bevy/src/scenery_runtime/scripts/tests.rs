//! Tests of the scenery scripts: the host values and overrides reach the script, and the string variables.

use super::*;


#[test]
fn host_values_and_overrides_reach_the_script() {
    let (engine, _) = ScriptEngine::from_sources(&[(
        "t.osc",
        "{frame}\n (L.S.Time) (S.L.t)\n (L.L.TrafficLightPhase) (S.L.p)\n \
         (L.L.NightlightA) (S.L.n)\n (L.S.Month) (S.L.m)\n (L.L.knob) (S.L.k)\n{end}",
    )]);
    let mut s = SceneryScripts::new(engine, &[], &[], Vec::new());
    let clock = SceneryClock::at(2026, 12, 24, 22.0 * 3600.0);
    let mut overrides = SceneryVariableOverrides::default();
    overrides.0.insert("Knob".into(), 4.0);
    s.tick(&HostValues::of(&clock), Some(7.0), &overrides, 0.1);
    let v = s
        .with_engine(|e| [e.get("t"), e.get("p"), e.get("n"), e.get("m"), e.get("k")])
        .unwrap();
    assert_eq!(v, [22.0 * 3600.0, 7.0, 1.0, 12.0, 4.0]);
    assert!(s.initialised && s.ticks == 1);
}

#[test]
fn string_variables_come_from_the_placed_object() {
    let (engine, _) = ScriptEngine::from_sources(&[("t.osc", "{init}\n{end}")]);
    let s = SceneryScripts::new(
        engine,
        &["Name".into(), "Line".into()],
        &["Hauptbahnhof".into()],
        Vec::new(),
    );
    let got = s
        .with_engine(|e| {
            (
                e.get_string("name").to_owned(),
                e.get_string("line").to_owned(),
            )
        })
        .unwrap();
    assert_eq!(got, ("Hauptbahnhof".to_string(), String::new()));
}
