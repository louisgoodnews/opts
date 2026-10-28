//! Unit tests for `super` (`run.rs`).

use super::*;

fn words(w: &[&str]) -> Vec<String> {
    w.iter().map(|s| (*s).to_string()).collect()
}

#[test]
fn spaced_command_names_resolve_to_dotted_methods() {
    let w = words(&["trace", "set", "full"]);
    let (m, rest) = split_method(&w).expect("method");
    assert_eq!((m.as_str(), rest), ("trace.set", &w[2..]));
    let w = words(&["state"]);
    assert_eq!(split_method(&w).expect("method").0, "state");
    let w = words(&["capture", "record", "start", "--fps", "15"]);
    let (m, rest) = split_method(&w).expect("method");
    assert_eq!((m.as_str(), rest.len()), ("capture.record.start", 2));
    let w = words(&["no", "such", "thing"]);
    assert_eq!(split_method(&w).expect("method").0, "no");
}

#[test]
fn a_cli_recording_defaults_to_30_seconds() {
    let mut p = serde_json::Value::Null;
    apply_cli_defaults("capture.record.start", &mut p);
    assert_eq!(p["duration_ms"], 30_000);
    let mut p = serde_json::json!({"duration_ms": 5000});
    apply_cli_defaults("capture.record.start", &mut p);
    assert_eq!(p["duration_ms"], 5000);
    let mut p = serde_json::Value::Null;
    apply_cli_defaults("state", &mut p);
    assert!(p.is_null());
}
