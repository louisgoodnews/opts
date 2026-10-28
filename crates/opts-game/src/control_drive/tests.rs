//! Unit tests for `super` (`control_drive.rs`).

use super::*;
use opts_bevy::vehicle_triggers::ScriptKeyBinding;

#[test]
fn aliases_resolve_only_when_the_vehicle_does_not_bind_the_name() {
    let bindings = ScriptKeyBindings(vec![ScriptKeyBinding {
        trigger: "bus_doorfront0".into(),
        key: KeyCode::KeyH,
        shift: false,
        ctrl: false,
        held: false,
    }]);
    assert_eq!(resolve_trigger(&bindings, "door_front"), "bus_doorfront0");
    assert_eq!(
        resolve_trigger(&bindings, "bus_doorfront0"),
        "bus_doorfront0"
    );
    assert_eq!(
        resolve_trigger(&bindings, "my_own_trigger"),
        "my_own_trigger"
    );
}

#[test]
fn numeric_values_are_range_checked() {
    assert!(number_value(&json!({"value": 0.5}), "throttle", 0.0, 1.0).is_ok());
    assert!(number_value(&json!({"value": 1.5}), "throttle", 0.0, 1.0).is_err());
    assert!(number_value(&json!({"value": "x"}), "steer", -1.0, 1.0).is_err());
    assert_eq!(
        number_value(&json!({"value": 2.0}), "steer", -1.0, 1.0)
            .unwrap_err()
            .code,
        code::INVALID_PARAMS
    );
}
