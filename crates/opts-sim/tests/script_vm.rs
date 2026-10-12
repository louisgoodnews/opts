//! Script VM: the test cases of `docs/src/formats/text/vehicles/osc.md` plus engine behaviour.

use opts_sim::script::{Curve, EventKind, ScriptEngine, Severity};

/// Compiles `body` as the `{init}` block, runs it and returns the engine.
fn run(body: &str) -> ScriptEngine {
    let (mut e, diags) =
        ScriptEngine::from_sources(&[("t.osc", &format!("{{init}}\n{body}\n{{end}}"))]);
    assert!(
        diags.iter().all(|d| d.severity != Severity::Error),
        "{diags:?}"
    );
    e.run_init();
    e
}

#[test]
fn t_osc_001_arithmetic_and_store() {
    assert_eq!(run("2 3 + (S.L.x)").get("x"), 5.0);
}

#[test]
fn t_osc_001b_store_keeps_the_value_for_chained_stores() {
    let e = run("7 (S.L.a) (S.L.b) (S.L.c)");
    assert_eq!((e.get("a"), e.get("b"), e.get("c")), (7.0, 7.0, 7.0));
    // Strings chain the same way.
    let e = run("\"hi\" (S.$.s1) (S.$.s2)");
    assert_eq!(e.get_string("s1"), "hi");
    assert_eq!(e.get_string("s2"), "hi");
}

#[test]
fn t_osc_002_if_taken() {
    assert_eq!(run("1 {if} 7 (S.L.x) {endif}").get("x"), 7.0);
}

#[test]
fn t_osc_003_else_branch() {
    assert_eq!(run("0 {if} 1 {else} 2 {endif} (S.L.x)").get("x"), 2.0);
}

#[test]
fn t_osc_004_comment_only_compiles_to_nothing() {
    let (e, d) = ScriptEngine::from_sources(&[("t.osc", "'only a comment")]);
    assert!(d.is_empty());
    drop(e);
}

#[test]
fn t_osc_005_registers() {
    let e = run("5 s3 l3 l3 + (S.L.x)");
    assert_eq!(e.get("x"), 10.0);
}

#[test]
fn t_osc_006_string_concat() {
    assert_eq!(run("\"a\" \"b\" $+ (S.$.s)").get_string("s"), "ab");
}

#[test]
fn t_osc_007_missing_endif_is_closed_at_end_with_a_warning() {
    // OMSI itself accepts these blocks (60+ corpus scripts), so they are not disabled.
    let (mut e, d) = ScriptEngine::from_sources(&[("t.osc", "{init}\n1 {if}\n2 (S.L.x)\n{end}")]);
    let w = d
        .iter()
        .find(|d| d.message.contains("without {endif}"))
        .expect("warning");
    assert_eq!(
        (w.severity, w.file.as_str(), w.line),
        (Severity::Warning, "t.osc", 2)
    );
    e.run_init();
    assert_eq!(e.get("x"), 2.0);
}

#[test]
fn surplus_endif_and_else_are_ignored_with_a_warning() {
    let (mut e, d) = ScriptEngine::from_sources(&[(
        "t.osc",
        "{init}\n1 {if} 1 (S.L.x) {endif} {endif} {else}\n{end}",
    )]);
    assert_eq!(
        d.iter().filter(|d| d.severity == Severity::Warning).count(),
        2
    );
    e.run_init();
    assert_eq!(e.get("x"), 1.0);
}

#[test]
fn nested_conditionals_and_logic() {
    let e = run(
        "1 {if} 0 {if} 1 (S.L.a) {else} 2 (S.L.a) {endif} 3 (S.L.b) {else} 9 (S.L.a) {endif}\n\
         (L.L.a) 2 = (L.L.b) 3 = && (S.L.ok) 1 !  (S.L.not1)",
    );
    assert_eq!(
        (e.get("a"), e.get("b"), e.get("ok"), e.get("not1")),
        (2.0, 3.0, 1.0, 0.0)
    );
}

#[test]
fn operators_follow_postfix_order() {
    let e = run("10 4 - (S.L.sub) 9 2 / (S.L.div) 7 3 % (S.L.m) 3 /-/ abs sqr (S.L.sq) 4 sqrt 1 max (S.L.mx) 5 0 / (S.L.dz)");
    assert_eq!(
        [
            e.get("sub"),
            e.get("div"),
            e.get("m"),
            e.get("sq"),
            e.get("mx"),
            e.get("dz")
        ],
        [6.0, 4.5, 1.0, 9.0, 2.0, 0.0]
    );
}

#[test]
fn macros_can_be_called_before_they_are_defined_and_across_files() {
    let (mut e, d) = ScriptEngine::from_sources(&[
        ("main.osc", "{init}\n2 (M.L.Double) (S.L.r)\n{end}"),
        ("lib.osc", "{macro:DOUBLE}\n2 *\n{end}"),
    ]);
    assert!(d.is_empty(), "{d:?}");
    e.run_init();
    assert_eq!(e.get("r"), 4.0);
}

#[test]
fn macros_share_the_registers_of_their_caller() {
    // Macros take arguments and return results in the registers (`bremse_druckausgleich`).
    let (mut e, _) = ScriptEngine::from_sources(&[(
        "t.osc",
        "{macro:m}\nl0 2 * s1\n{end}\n{init}\n5 s0 (M.L.m) l1 (S.L.x)\n{end}",
    )]);
    e.run_init();
    assert_eq!(e.get("x"), 10.0);
}

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

#[test]
fn string_operators_match_corpus_usage() {
    let e = run("\"abcdef\" 2 $SetLengthL (S.$.a)\n\
         \"abcdef\" 2 $SetLengthR (S.$.b)\n\
         7 \"02\" $IntToStrEnh \":\" $+ (S.$.c)\n\
         \" \" 3 $* \"x\" $+ (S.$.d)\n\
         \"abc\" $length (S.L.len)\n\
         \"x\" $d $= (S.L.same)\n\
         \"3.5\" $StrToFloat (S.L.f)\n\
         \"abcdef\" 1 $cutBegin 2 $cutEnd (S.$.e)");
    assert_eq!(
        [
            e.get_string("a"),
            e.get_string("b"),
            e.get_string("c"),
            e.get_string("d"),
            e.get_string("e")
        ],
        ["ab", "ef", "07:", "   x", "bcd"]
    );
    assert_eq!((e.get("len"), e.get("same"), e.get("f")), (3.0, 1.0, 3.5));
}

#[test]
fn msg_goes_to_the_console_buffer() {
    let mut e = run("\"hello\" $msg");
    assert_eq!(e.drain_messages(), ["hello"]);
}

#[test]
fn problems_are_leniently_ignored_and_reported() {
    let (mut e, d) = ScriptEngine::from_sources(&[(
        "t.osc",
        "{init}\n+ (S.L.u)\n(M.L.nowhere)\nbogus 4 (S.L.ok)\n{end}",
    )]);
    assert!(d.iter().any(|d| d.message.contains("bogus") && d.line == 4));
    e.run_init();
    assert_eq!(e.get("ok"), 4.0);
    assert_eq!(e.get("u"), 0.0);
    let run_time = e.drain_diagnostics();
    assert!(run_time.iter().any(|d| d.message.contains("underflow")));
    assert!(run_time.iter().any(|d| d.message.contains("nowhere")));
    e.run_init();
    assert!(
        e.drain_diagnostics().is_empty(),
        "each problem is reported once"
    );
}

#[test]
fn external_macros_reach_the_host() {
    let (mut e, _) =
        ScriptEngine::from_sources(&[("t.osc", "{init}\n2 3 (M.V.GiveChangeCoin)\n{end}")]);
    e.set_external_handler(Box::new(|name, st| {
        assert_eq!(name, "givechangecoin");
        let b = st.numbers.pop().unwrap_or(0.0);
        let a = st.numbers.pop().unwrap_or(0.0);
        st.numbers.push(a * b);
    }));
    e.run_init();
}

#[test]
fn random_is_deterministic_for_a_seed() {
    let src = [("t.osc", "{init}\n1 random (S.L.a) 1 random (S.L.b)\n{end}")];
    let mut values = Vec::new();
    for _ in 0..2 {
        let (mut e, _) = ScriptEngine::from_sources(&src);
        e.seed_random(42);
        e.run_init();
        values.push((e.get("a"), e.get("b")));
    }
    assert_eq!(values[0], values[1]);
    assert!((0.0..1.0).contains(&values[0].0) && values[0].0 != values[0].1);
}

#[test]
fn runaway_recursion_is_cut_off() {
    let (mut e, _) = ScriptEngine::from_sources(&[(
        "t.osc",
        "{macro:loop}\n(L.L.n) 1 + (S.L.n) (M.L.loop)\n{end}\n{init}\n(M.L.loop)\n{end}",
    )]);
    e.run_init();
    assert_eq!(e.get("n"), 63.0);
    assert!(e
        .drain_diagnostics()
        .iter()
        .any(|d| d.message.contains("too deep")));
}

#[test]
fn missing_end_still_compiles_with_a_warning() {
    let (mut e, d) = ScriptEngine::from_sources(&[("t.osc", "{init}\n1 (S.L.x)")]);
    assert!(d
        .iter()
        .any(|d| d.severity == Severity::Warning && d.message.contains("{end}")));
    e.run_init();
    assert_eq!(e.get("x"), 1.0);
}

#[test]
fn vehicle_loader_reads_scripts_varlists_and_constfiles() {
    use opts_core::formats::vehicles::BusManifest;
    use opts_sim::script::load_vehicle_scripts;

    let dir = std::env::temp_dir().join(format!("opts-script-load-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("mkdir");
    std::fs::write(
        dir.join("main.osc"),
        "{frame}\n(C.L.k) (L.L.in) * (S.L.out)\n{end}",
    )
    .expect("write");
    std::fs::write(dir.join("v.txt"), "in\nout\n").expect("write");
    std::fs::write(dir.join("c.txt"), "prose\n[const]\nk\n3\n").expect("write");
    let bus = BusManifest::parse(
        "[varnamelist]\n1\nv.txt\n[script]\n2\nmain.osc\nmissing.osc\n[constfile]\n1\nc.txt\n",
    )
    .expect("bus");
    let (mut e, diags) = load_vehicle_scripts(&bus, |rel| {
        let p = dir.join(rel);
        p.exists().then_some(p)
    });
    assert_eq!(
        diags
            .iter()
            .filter(|d| d.message == "file not found")
            .count(),
        1
    );
    assert!(e.undeclared_variables().is_empty());
    e.set("in", 4.0);
    e.run_frame(0.016, false);
    assert_eq!(e.get("out"), 12.0);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn t_osc_001c_register_store_keeps_the_value() {
    // `5 s0 3 +` leaves 8 (5 stays below the 3 it is added to... the store did not pop).
    let e = run("2 s0 3 + (S.L.x) l0 (S.L.y)");
    assert_eq!(e.get("x"), 5.0);
    assert_eq!(e.get("y"), 2.0);
    let e = run("4 s1 (L.L.z) + (S.L.x)");
    assert_eq!(
        e.get("x"),
        4.0,
        "the stored 4 is still there to add to z (= 0)"
    );
}

#[test]
fn random_scales_by_the_value_below_it() {
    let e = run("1000 random (S.L.x)");
    assert!((0.0..1000.0).contains(&e.get("x")));
    let e = run("0 random (S.L.x)");
    assert_eq!(e.get("x"), 0.0);
}
