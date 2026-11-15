//! surplus `endif`, nested conditionals, operator order and macros.

use opts_sim::script::{ScriptEngine, Severity};

use super::*;

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
