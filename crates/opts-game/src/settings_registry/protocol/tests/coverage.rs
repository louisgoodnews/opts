//! The coverage tests: every registered setting is reachable through `settings.*`, and every typed
//! stored key is registered.

use super::*;

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
