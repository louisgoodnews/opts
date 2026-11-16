//! Stepping through values, and the restart marker.

use super::*;

#[test]
fn stepping_toggles_a_switch_wraps_a_choice_and_clamps_a_number() {
    let make = |kind, values: &[i32], min_val, max_val| SetvarItem {
        name: "x".into(),
        label: "X".into(),
        description: String::new(),
        value: 0,
        default_value: 0,
        min_val,
        max_val,
        kind,
        values: values.to_vec(),
        group: None,
        unverified: false,
        respawn: Respawn::Unknown,
    };
    let switch = make(SetvarKind::Switch, &[], 0, 1);
    assert_eq!((switch.step(0, 1), switch.step(1, 1)), (1, 0));
    let choice = make(SetvarKind::Choice, &[0, 2, 5], 0, 5);
    assert_eq!(choice.step(2, 1), 5);
    assert_eq!(choice.step(5, 1), 0, "wraps forward");
    assert_eq!(choice.step(0, -1), 5, "wraps backward");
    assert_eq!(
        choice.step(3, 1),
        2,
        "an unlisted value starts at the first"
    );
    let number = make(SetvarKind::Number, &[], 0, 10);
    assert_eq!(
        (number.step(10, 1), number.step(0, -1), number.step(4, 1)),
        (10, 0, 5)
    );
}

#[test]
fn an_option_the_vendor_says_needs_a_restart_carries_the_marker() {
    let mut r = reps(&[
        ("Red", "engine_sv_req", "1"),
        ("Red", "sv_display_back", "1"),
    ]);
    r.folder = "CW_IVECOBUS_INTER_FAMILY".into();
    for name in ["engine_sv_req", "sv_display_back"] {
        r.declared
            .entry(name.to_string())
            .or_default()
            .insert("setvar_list.txt".to_string());
    }
    let items = setvars_from(&r, &HashMap::new(), "Red");
    assert_eq!(item(&items, "engine_sv_req").respawn, Respawn::Required);
    assert_eq!(item(&items, "sv_display_back").respawn, Respawn::Unknown);
    // No documented family: nothing is claimed.
    let plain = setvars_from(&reps(&[("Red", "x", "1")]), &HashMap::new(), "Red");
    assert_eq!(item(&plain, "x").respawn, Respawn::Unknown);
}
