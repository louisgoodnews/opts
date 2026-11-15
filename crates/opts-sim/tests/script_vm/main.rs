//! Script VM: the test cases of `docs/src/formats/text/vehicles/osc.md` plus engine behaviour.

use opts_sim::script::{ScriptEngine, Severity};

mod control;
mod engine;
mod host;
mod language;
mod loader;

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
