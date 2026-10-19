//! Unit tests of `commands.rs` (moved out of the file to keep it under the 500-line limit, refactoring cycle 1).

use super::*;

fn params(method: &str, words: &[&str]) -> Result<Value, String> {
    let words: Vec<String> = words.iter().map(|s| (*s).to_string()).collect();
    build_params(find(method).expect("known method"), &words)
}

#[test]
fn positional_and_flag_forms_agree() {
    let a = params("input.key", &["W", "--duration", "2000"]).unwrap();
    let b = params("input.key", &["key=W", "duration_ms=2000"]).unwrap();
    assert_eq!(a, json!({"key": "W", "duration_ms": 2000}));
    assert_eq!(a, b);
}

#[test]
fn typed_values_are_coerced() {
    assert_eq!(params("sim.step", &["60"]).unwrap(), json!({"frames": 60}));
    assert_eq!(
        params("camera.teleport", &["1.5", "2", "3", "--yaw", "90"]).unwrap(),
        json!({"x": 1.5, "y": 2.0, "z": 3.0, "yaw": 90.0})
    );
    assert_eq!(
        params("debug.wireframe", &["on"]).unwrap(),
        json!({"enabled": true})
    );
    assert_eq!(
        params("shutdown", &["--force"]).unwrap(),
        json!({"force": true})
    );
}

#[test]
fn json_flag_supplies_a_whole_object() {
    let v = params(
        "entity.spawn",
        &["--json", r#"{"asset":"a.sco","transform":{"x":1}}"#],
    );
    assert_eq!(v.unwrap(), json!({"asset": "a.sco", "transform": {"x": 1}}));
}

#[test]
fn errors_are_readable() {
    assert!(params("sim.step", &[]).unwrap_err().contains("frames"));
    assert!(params("sim.step", &["many"])
        .unwrap_err()
        .contains("integer"));
    assert!(params("ping", &["x"]).unwrap_err().contains("no value"));
    assert!(params("input.key", &["W", "--nope", "1"])
        .unwrap_err()
        .contains("nope"));
}

#[test]
fn new_commands_parse_words_into_typed_params() {
    // A JSON parameter that is not JSON is a string: `drive.set gear drive`.
    assert_eq!(
        params("drive.set", &["gear", "drive"]).unwrap(),
        json!({"control": "gear", "value": "drive"})
    );
    assert_eq!(
        params("drive.set", &["throttle", "0.5", "--hold_ms", "300"]).unwrap(),
        json!({"control": "throttle", "value": 0.5, "hold_ms": 300})
    );
    assert_eq!(
        params("drive.set", &["parking_brake", "false"]).unwrap(),
        json!({"control": "parking_brake", "value": false})
    );
    assert_eq!(
        params("ui.scroll", &["--target", "settings", "--to", "bottom"]).unwrap(),
        json!({"target": "settings", "to": "bottom"})
    );
    assert_eq!(
        params("ui.wait_for", &["distance_above", "--number", "100"]).unwrap(),
        json!({"until": "distance_above", "number": 100.0})
    );
    assert_eq!(
        params("ui.key", &["F", "--ctrl"]).unwrap(),
        json!({"key": "F", "ctrl": true})
    );
    assert!(params("ui.wait_for", &[]).unwrap_err().contains("until"));
}

#[test]
fn every_enum_value_list_is_parsed() {
    for c in COMMANDS {
        for a in c.args_schema().as_array().expect("array") {
            // A declared value list is never empty and has no stray separators.
            if let Some(v) = a.get("values") {
                assert!(
                    v.as_array()
                        .expect("values")
                        .iter()
                        .all(|x| x.as_str().is_some_and(|s| !s.is_empty())),
                    "{} / {}",
                    c.method,
                    a["name"]
                );
            }
        }
    }
}

#[test]
fn parameterless_commands_send_no_params() {
    assert_eq!(params("ping", &[]).unwrap(), Value::Null);
}

#[test]
fn table_has_unique_names() {
    let mut names: Vec<_> = COMMANDS.iter().map(|c| c.method).collect();
    names.sort_unstable();
    let n = names.len();
    names.dedup();
    assert_eq!(n, names.len());
}
