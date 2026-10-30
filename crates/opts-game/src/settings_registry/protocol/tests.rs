//! The settings protocol beyond the basics (M-0072): every registered setting is reachable
//! through `settings.*` (enumerated from the registry, so a new setting is covered by itself),
//! the typed stored keys are all registered, risky settings can wait for confirmation, a batch is
//! atomic, and a profile survives export and import.

use bevy::prelude::*;
use opts_bevy::control::state::Outcome;
use opts_cli::protocol::{code, RpcError};
use serde_json::{json, Value};

use crate::settings::GameSettings;
use crate::settings_registry::protocol::{get, list, reset, set};
use crate::settings_registry::protocol_confirm::{confirm, revert, tick_by, ProtocolRevert};
use crate::settings_registry::{
    normalize, Apply, Available, SettingChanged, SettingDescriptor, SettingKind,
    SettingsCapabilities, SettingsRegistry,
};

fn world(name: &str) -> World {
    let mut w = World::new();
    let dir = std::env::temp_dir().join(format!("opts-settings-cov-{}-{name}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let mut s = GameSettings::default();
    s.db_path_override = Some(dir.join("settings.db"));
    w.insert_resource(s);
    w.insert_resource(SettingsRegistry::with_builtin());
    w.insert_resource(SettingsCapabilities::default());
    w.init_resource::<Messages<SettingChanged>>();
    w
}

fn ok(o: Outcome) -> Value {
    match o {
        Outcome::Done(r) => r.expect("ok"),
        Outcome::Wait(_) => panic!("answers at once"),
    }
}

fn err(o: Outcome) -> RpcError {
    match o {
        Outcome::Done(r) => r.expect_err("error"),
        Outcome::Wait(_) => panic!("answers at once"),
    }
}

fn descriptors(w: &World) -> Vec<SettingDescriptor> {
    w.resource::<SettingsRegistry>()
        .all()
        .into_iter()
        .copied()
        .collect()
}

/// The stored default of `d`, normalised like a stored value.
fn default_of(d: &SettingDescriptor) -> String {
    normalize(d, d.default).unwrap_or_else(|_| d.default.to_string())
}

/// A valid value for `d` that differs from `current`; the settings with their own rules are
/// listed here with the reason.
fn alternative(d: &SettingDescriptor, current: &str) -> Option<String> {
    match d.kind {
        SettingKind::Bool => Some((current != "true").to_string()),
        SettingKind::Enum(options) => options
            .iter()
            .map(|o| o.value)
            .find(|v| *v != current)
            .map(str::to_string),
        SettingKind::Int { min, max, .. } => {
            let now: i64 = current.parse().ok()?;
            Some(if now != min { min } else { max }.to_string())
        }
        SettingKind::Float { min, max, .. } => {
            let now: f64 = current.parse().ok()?;
            Some(if (now - min).abs() > 1e-9 { min } else { max }.to_string())
        }
        // The language is a locale tag the localisation must know.
        SettingKind::Text if d.key == "interface.language" => {
            Some(if current == "de" { "en" } else { "de" }.to_string())
        }
        SettingKind::Text => Some("opts-coverage-test".to_string()),
        SettingKind::Path => Some(std::env::temp_dir().to_string_lossy().into_owned()),
        SettingKind::Time => Some(if current == "06:45" { "07:15" } else { "06:45" }.to_string()),
        // Chosen by the caller: a key that is free depends on the other bindings.
        SettingKind::Binding => None,
        SettingKind::Action => None,
    }
}

/// Checks that one registered setting is listed, gettable, settable and resettable.
fn check_setting(w: &mut World, d: SettingDescriptor, listed: &[String], fails: &mut Vec<String>) {
    let key = d.key;
    if !listed.iter().any(|k| k == key) {
        fails.push(format!("{key}: not in settings.list"));
    }
    if let Outcome::Done(Err(e)) = get(w, &json!({ "key": key })) {
        fails.push(format!("{key}: settings.get failed: {}", e.message));
        return;
    }
    let usable = match d.available {
        Available::Always => true,
        Available::Capability(c) => w.resource::<SettingsCapabilities>().has(c),
        Available::NotYet(_) => false,
    };
    if !usable {
        let e = err(set(w, &json!({ "key": key, "value": d.default })));
        if e.code != code::NOT_READY {
            fails.push(format!("{key}: unavailable but set answered {}", e.code));
        }
        return;
    }
    let candidates: Vec<String> = match d.kind {
        // Keys that no default binding and no reserved key of OPTS uses; the first accepted
        // one is the test value.
        SettingKind::Binding => ["Numpad7", "Numpad8", "Numpad9", "F12"]
            .iter()
            .map(|k| (*k).to_string())
            .collect(),
        _ => alternative(&d, &default_of(&d)).into_iter().collect(),
    };
    let Some(alt) = candidates
        .iter()
        .find(|c| {
            let mut probe = world("probe");
            matches!(
                set(&mut probe, &json!({ "key": key, "value": c })),
                Outcome::Done(Ok(_))
            ) || !matches!(d.kind, SettingKind::Binding)
        })
        .cloned()
    else {
        if matches!(d.kind, SettingKind::Binding) {
            fails.push(format!("{key}: no free key found among the candidates"));
            return;
        }
        // Actions hold no value: setting one is refused.
        if matches!(
            set(w, &json!({ "key": key, "value": "x" })),
            Outcome::Done(Ok(_))
        ) {
            fails.push(format!("{key}: an action accepted a value"));
        }
        return;
    };
    match set(w, &json!({ "key": key, "value": alt })) {
        Outcome::Done(Ok(v)) => {
            let want = normalize(&d, &alt).unwrap_or_else(|_| alt.clone());
            if v["value"].as_str() != Some(want.as_str()) {
                fails.push(format!("{key}: set {alt:?} read back {:?}", v["value"]));
            }
        }
        Outcome::Done(Err(e)) => {
            fails.push(format!("{key}: set {alt:?} refused: {}", e.message));
            return;
        }
        Outcome::Wait(_) => panic!("answers at once"),
    }
    match reset(w, &json!({ "key": key })) {
        Outcome::Done(Ok(v)) => {
            if v["value"].as_str() != Some(default_of(&d).as_str()) {
                fails.push(format!("{key}: reset gave {:?}", v["value"]));
            }
        }
        Outcome::Done(Err(e)) => fails.push(format!("{key}: reset refused: {}", e.message)),
        Outcome::Wait(_) => panic!("answers at once"),
    }
}

#[test]
fn every_registered_setting_is_listed_gettable_settable_and_resettable() {
    let mut w = world("coverage");
    let listed = ok(list(&mut w, &json!({})));
    let listed_keys: Vec<String> = listed["settings"]
        .as_array()
        .expect("settings")
        .iter()
        .map(|s| s["key"].as_str().expect("key").to_string())
        .collect();
    let mut failures: Vec<String> = Vec::new();
    for d in descriptors(&w) {
        check_setting(&mut w, d, &listed_keys, &mut failures);
    }
    assert!(
        failures.is_empty(),
        "not reachable:\n{}",
        failures.join("\n")
    );
}

/// Stored keys that are program state, not preferences, so the registry has no entry for them.
const NOT_PREFERENCES: &[(&str, &str)] = &[];

#[test]
fn every_typed_stored_key_is_a_registered_setting() {
    let w = world("fields");
    let registry = w.resource::<SettingsRegistry>();
    let missing: Vec<&str> = GameSettings::FIELD_KEYS
        .iter()
        .copied()
        .filter(|k| registry.get(k).is_none())
        .filter(|k| !NOT_PREFERENCES.iter().any(|(n, _)| n == k))
        .collect();
    assert!(
        missing.is_empty(),
        "stored but not reachable through settings.*: {missing:?}"
    );
}

fn risky_key(w: &World) -> &'static str {
    descriptors(w)
        .into_iter()
        .find(|d| matches!(d.apply, Apply::WithRevert(_)))
        .expect("a risky setting")
        .key
}

fn other_value(w: &mut World, key: &str) -> String {
    let d = *w
        .resource::<SettingsRegistry>()
        .get(key)
        .expect("registered");
    let current = ok(get(w, &json!({ "key": key })))["value"]
        .as_str()
        .map(str::to_string)
        .unwrap_or_default();
    alternative(&d, &current).expect("an alternative")
}

fn value_of(w: &mut World, key: &str) -> String {
    ok(get(w, &json!({ "key": key })))["value"]
        .as_str()
        .unwrap_or_default()
        .to_string()
}

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

#[path = "tests/batch.rs"]
mod batch;
