//! operators, stores, conditionals and string length of the OSC language (`t_osc_001` to `t_osc_007`).

use opts_sim::script::{ScriptEngine, Severity};

use super::*;

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
fn t_osc_010_stores_keep_the_value_on_the_stack() {
    let e = run("4 s0 3 > (S.L.a) l0 (S.L.b) (S.L.c)");
    assert_eq!((e.get("a"), e.get("b"), e.get("c")), (1.0, 4.0, 4.0));
    let mut e = e;
    assert!(
        e.drain_diagnostics().is_empty(),
        "no stack underflow for the season-switch idiom"
    );
}

#[test]
fn t_osc_011_if_leaves_its_condition_on_the_stack() {
    // The ticket printer's power idiom: the busbar value stays after `{if}` and is compared with
    // the negated state (a popping `{if}` would never switch the printer on).
    let mut e = run("1 (S.L.busbar) {if} (L.L.an) ! = {if} 1 (S.L.an) {endif} {endif}");
    assert_eq!(e.get("an"), 1.0);
    // Once on, `!an` is 0, the comparison fails and the state stays.
    let e2 = run("1 (S.L.an) 1 {if} (L.L.an) ! = {if} 5 (S.L.hit) {endif} {endif}");
    assert_eq!(e2.get("hit"), 0.0);
    assert!(e.drain_diagnostics().is_empty(), "no stack underflow");
}

#[test]
fn t_osc_012_length_leaves_the_string_and_a_non_number_is_minus_one() {
    // `(S.$.x) $length (S.L.n) 0 $* $+`: store, measure, drop (the matrix script's idiom).
    let e = run("\"abc\" (S.$.t) $length (S.L.n) 0 $* $+ (S.$.after)");
    assert_eq!(e.get("n"), 3.0);
    assert_eq!(e.get_string("t"), "abc");
    let e =
        run("\"abc\" $StrToFloat (S.L.a) \"\" $StrToFloat (S.L.b) \"12.5\" $StrToFloat (S.L.c)");
    assert_eq!((e.get("a"), e.get("b"), e.get("c")), (-1.0, -1.0, 12.5));
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
