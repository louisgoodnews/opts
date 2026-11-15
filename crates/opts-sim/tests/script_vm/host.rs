//! string operators, console messages, problems, host macros, randomness and recursion limits.

use opts_sim::script::{ScriptEngine, Severity};

use super::*;

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
        true
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
