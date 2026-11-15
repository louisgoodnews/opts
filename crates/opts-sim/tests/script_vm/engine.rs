//! frames, triggers, constants and curves of the engine.

use opts_sim::script::{Curve, EventKind, ScriptEngine};


#[test]
fn frame_sets_timegap_and_accumulates_time() {
    let (mut e, _) = ScriptEngine::from_sources(&[(
        "t.osc",
        "{frame}\n(L.L.odo) (L.S.Timegap) 10 * + (S.L.odo) (L.S.GetTime) (S.L.t)\n{end}\n\
         {frame_ai}\n1 (S.L.ai)\n{end}",
    )]);
    e.run_frame(0.5, false);
    e.run_frame(0.5, false);
    assert_eq!((e.get("odo"), e.get("t"), e.get("ai")), (10.0, 1.0, 0.0));
    e.run_frame(0.1, true);
    assert_eq!(e.get("ai"), 1.0);
}

#[test]
fn triggers_run_and_raise_events() {
    let (mut e, _) = ScriptEngine::from_sources(&[(
        "lights.osc",
        "{trigger:blinker_left_move}\n(L.L.sw) 0 =\n{if}\n1 (S.L.sw)\n(T.L.ev_swon)\n{else}\n(T.L.ev_swoff)\n0 (S.L.sw)\n{endif}\n{end}",
    )]);
    assert!(e.trigger("Blinker_Left_Move"));
    assert!(!e.trigger("unknown"));
    assert_eq!(e.get("sw"), 1.0);
    assert!(e.trigger("blinker_left_move"));
    let names: Vec<_> = e
        .drain_events()
        .into_iter()
        .map(|v| (v.kind, v.name))
        .collect();
    assert_eq!(
        names,
        [
            (EventKind::Local, "ev_swon".to_owned()),
            (EventKind::Local, "ev_swoff".to_owned())
        ]
    );
    assert!(e.drain_events().is_empty());
}

#[test]
fn constants_and_curves() {
    let (mut e, _) = ScriptEngine::from_sources(&[(
        "t.osc",
        "{init}\n(C.L.Wattage) (S.L.w)\n5 (F.L.Damper) (S.L.y)\n1 (C.L.missing) + (S.L.z)\n{end}",
    )]);
    e.consts_mut().set_scalar("wattage", 55.0);
    e.consts_mut()
        .set_curve("damper", Curve::new(vec![(0.0, 0.0), (10.0, 100.0)]));
    e.run_init();
    assert_eq!((e.get("w"), e.get("y"), e.get("z")), (55.0, 50.0, 1.0));
    let warnings = e.drain_diagnostics();
    assert_eq!(
        warnings.len(),
        1,
        "an undefined constant is reported once: {warnings:?}"
    );
}
