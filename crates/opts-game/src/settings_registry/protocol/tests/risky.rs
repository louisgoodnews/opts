//! The risky settings: a change waits for its confirmation, and only one such change waits at a time.

use super::*;

#[test]
fn a_risky_setting_waits_for_confirm_and_can_be_kept_or_reverted() {
    let mut w = world("confirm");
    let key = risky_key(&w);
    let before = value_of(&mut w, key);
    let alt = other_value(&mut w, key);

    // Kept: applied at once, saved after `settings.confirm`.
    let v = ok(set(
        &mut w,
        &json!({ "key": key, "value": alt, "confirm": false }),
    ));
    assert_eq!(v["pending_confirm"], true);
    assert!(v["confirm_within_s"].as_u64().is_some_and(|s| s > 0));
    assert_eq!(value_of(&mut w, key), alt, "applied while it waits");
    assert!(w.contains_resource::<ProtocolRevert>());
    let c = ok(confirm(&mut w, &json!({})));
    assert_eq!(
        (c["key"].as_str(), c["confirmed"].as_bool()),
        (Some(key), Some(true))
    );
    assert!(!w.contains_resource::<ProtocolRevert>());
    assert_eq!(value_of(&mut w, key), alt);

    // Reverted by hand.
    let v = ok(set(
        &mut w,
        &json!({ "key": key, "value": before, "confirm": false }),
    ));
    assert_eq!(v["pending_confirm"], true);
    let r = ok(revert(&mut w, &json!({})));
    assert_eq!(r["reverted"], true);
    assert_eq!(value_of(&mut w, key), alt, "back to the confirmed value");

    // Reverted by the timeout.
    ok(set(
        &mut w,
        &json!({ "key": key, "value": before, "confirm": false }),
    ));
    assert!(!tick_by(&mut w, 5.0), "still waiting");
    assert!(w.contains_resource::<ProtocolRevert>());
    assert!(tick_by(&mut w, 60.0), "the timeout reverts");
    assert!(!w.contains_resource::<ProtocolRevert>());
    assert_eq!(value_of(&mut w, key), alt);
}

#[test]
fn only_one_risky_change_waits_and_nothing_waiting_is_not_ready() {
    let mut w = world("busy");
    let key = risky_key(&w);
    let alt = other_value(&mut w, key);
    assert_eq!(err(confirm(&mut w, &json!({}))).code, code::NOT_READY);
    assert_eq!(err(revert(&mut w, &json!({}))).code, code::NOT_READY);
    ok(set(
        &mut w,
        &json!({ "key": key, "value": alt, "confirm": false }),
    ));
    let again = err(set(
        &mut w,
        &json!({ "key": key, "value": "x", "confirm": false }),
    ));
    assert_eq!(again.code, code::BUSY);
    ok(revert(&mut w, &json!({})));
}

#[test]
fn confirm_false_changes_nothing_for_settings_without_a_prompt() {
    let mut w = world("plain");
    let v = ok(set(
        &mut w,
        &json!({ "key": "look", "value": "natural", "confirm": false }),
    ));
    assert!(v.get("pending_confirm").is_none());
    assert!(!w.contains_resource::<ProtocolRevert>());
    assert_eq!(value_of(&mut w, "look"), "natural");
    assert_eq!(
        err(set(
            &mut w,
            &json!({ "key": "look", "value": "vivid", "confirm": "no" })
        ))
        .code,
        code::INVALID_PARAMS
    );
}

#[test]
fn a_change_that_needs_a_restart_says_so() {
    let mut w = world("restart");
    let restart = descriptors(&w)
        .into_iter()
        .find(|d| matches!(d.apply, Apply::OnRestart) && matches!(d.available, Available::Always))
        .expect("a restart setting")
        .key;
    let alt = other_value(&mut w, restart);
    let v = ok(set(&mut w, &json!({ "key": restart, "value": alt })));
    assert_eq!(
        (v["applied"].as_bool(), v["needs_restart"].as_bool()),
        (Some(true), Some(true))
    );
    let same = ok(set(&mut w, &json!({ "key": restart, "value": alt })));
    assert_eq!(
        (same["applied"].as_bool(), same["needs_restart"].as_bool()),
        (Some(false), Some(false)),
        "no change, nothing to restart"
    );
    let live = ok(set(&mut w, &json!({ "key": "look", "value": "natural" })));
    assert_eq!(live["needs_restart"], false);
}
