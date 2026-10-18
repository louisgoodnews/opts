//! M-0096: a callback the host does not implement never misaligns a script's stacks. For every
//! callback of OMSI's lists a script pushes a sentinel, the arguments of the signature, calls it
//! and takes the results off again; the sentinel must still be there and no run-time diagnostic
//! may complain about a stack.

use opts_sim::script::callbacks::{CallbackSignature, CALLBACKS};
use opts_sim::script::ScriptEngine;

/// The script of one callback: the arguments, the call, then a probe of the float stack's depth.
/// With `d` values on the stack `d - 1` additions succeed and the `d`-th underflows, so exactly
/// one underflow diagnostic means that exactly the declared float results are left (a leftover
/// argument would let the last addition succeed, a missing result would underflow earlier). A
/// string stack underflow is silent in the VM; the string results are checked by the unit test of
/// `CallbackSignature::apply_stub`.
fn script(sig: &CallbackSignature) -> String {
    let mut s = String::from("{init}\n");
    for _ in 0..sig.float_args {
        s.push_str("5 ");
    }
    for _ in 0..sig.string_args {
        s.push_str("\"x\" ");
    }
    s.push_str(&format!("(M.V.{})\n", sig.name));
    for _ in 0..sig.float_results.saturating_sub(1) {
        s.push_str("+ ");
    }
    s.push_str("+\n{end}\n");
    s
}

#[test]
fn every_callback_stub_leaves_exactly_its_float_results_on_the_stack() {
    for sig in CALLBACKS {
        let text = script(sig);
        let (mut e, compile) = ScriptEngine::from_sources(&[("t.osc", &text)]);
        assert!(compile.is_empty(), "{}: {compile:?}\n{text}", sig.name);
        e.run_init();
        let diags = e.drain_diagnostics();
        let underflows = diags
            .iter()
            .filter(|d| d.message.contains("underflow"))
            .count();
        assert_eq!(underflows, 1, "{}: {diags:?}\n{text}", sig.name);
        // Counted as stubbed, reported once.
        assert_eq!(e.callback_stats().stub.get(sig.name).copied(), Some(1));
        assert!(
            diags.iter().any(|d| d.message.contains("neutral stub")),
            "{}: {diags:?}",
            sig.name
        );
    }
}

#[test]
fn a_callback_the_host_answers_is_counted_as_real_and_a_stranger_as_unknown() {
    let (mut e, _) = ScriptEngine::from_sources(&[(
        "t.osc",
        "{init}\n3 (M.V.NrSpecRandom) (S.L.r)\n1 (M.V.MadeUpByNobody)\n{end}",
    )]);
    e.set_external_handler(Box::new(|name, st| {
        if name == "nrspecrandom" {
            st.numbers.pop();
            st.numbers.push(0.25);
            true
        } else {
            false
        }
    }));
    e.run_init();
    assert_eq!(e.get("r"), 0.25);
    let stats = e.callback_stats();
    assert_eq!(stats.real.get("nrspecrandom"), Some(&1));
    assert_eq!(stats.unknown.get("madeupbynobody"), Some(&1));
    assert!(stats.stub.is_empty());
}

#[test]
fn the_usage_lists_what_the_scripts_read_write_and_call() {
    let (e, _) = ScriptEngine::from_sources(&[(
        "t.osc",
        "{frame}\n(L.L.velocity) 2 * (S.L.m_wheel)\n(L.S.timeofday)\n1 (M.V.GetTTDelay) (S.L.d)\n\"a\" (S.$.s)\n{end}",
    )]);
    let u = e.usage();
    assert!(u.reads.contains("velocity"));
    assert!(u.writes.contains("m_wheel") && u.writes.contains("d"));
    assert!(u.system_reads.contains("timeofday"));
    assert!(u.external.contains("getttdelay"));
    assert!(u.string_writes.contains("s"));
}
