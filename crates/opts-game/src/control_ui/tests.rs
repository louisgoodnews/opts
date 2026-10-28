//! Unit tests for `super` (`control_ui.rs`).

use super::*;

fn cond(until: &str) -> Cond {
    Cond {
        until: until.into(),
        id: None,
        value: None,
        number: None,
    }
}

#[test]
fn conditions_are_validated() {
    assert!(validate(&cond("nonsense")).is_err());
    assert!(validate(&cond("visible")).is_err(), "needs an id");
    let mut c = cond("visible");
    c.id = Some("x".into());
    assert!(validate(&c).is_ok());
    assert!(validate(&cond("speed_above")).is_err(), "needs a number");
    assert!(validate(&cond("hud_ready")).is_ok());
}

#[test]
fn every_known_condition_is_documented_in_the_command_table() {
    let spec = opts_cli::commands::find("ui.wait_for").expect("descriptor");
    let args = spec.args_schema();
    let values: Vec<String> = args[0]["values"]
        .as_array()
        .expect("values")
        .iter()
        .filter_map(|v| v.as_str().map(str::to_string))
        .collect();
    for k in KNOWN {
        assert!(
            values.contains(&(*k).to_string()),
            "`{k}` missing in the descriptor"
        );
    }
    assert_eq!(values.len(), KNOWN.len());
}
