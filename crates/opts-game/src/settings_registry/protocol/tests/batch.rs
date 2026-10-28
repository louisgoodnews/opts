//! Tests for atomic batch updates, diffing, export and import of settings profiles.

use super::*;
use crate::settings_registry::protocol_batch::{diff, export, import, set_many};

#[test]
fn a_batch_is_applied_whole_or_not_at_all() {
    let mut w = world("batch");
    let bad = err(set_many(
        &mut w,
        &json!({ "values": { "look": "natural", "ui_scale": "9999", "no.such": 1 } }),
    ));
    assert_eq!(bad.code, code::INVALID_PARAMS);
    let errors = bad
        .data
        .as_ref()
        .and_then(|d| d["errors"].as_array())
        .expect("errors");
    let keys: Vec<&str> = errors.iter().filter_map(|e| e["key"].as_str()).collect();
    assert_eq!(keys, vec!["no.such", "ui_scale"], "every refusal is listed");
    assert_eq!(value_of(&mut w, "look"), "vivid", "nothing changed");

    let done = ok(set_many(
        &mut w,
        &json!({ "values": { "look": "natural", "ui_scale": 150, "high_contrast": false } }),
    ));
    assert_eq!(done["count"], 3);
    assert_eq!(done["changed"].as_array().map(Vec::len), Some(2));
    assert_eq!(done["unchanged"], 1);
    assert_eq!(value_of(&mut w, "look"), "natural");
    assert_eq!(w.resource::<GameSettings>().ui_scale, 150);
    assert_eq!(w.resource::<Messages<SettingChanged>>().len(), 2);

    assert_eq!(
        err(set_many(&mut w, &json!({ "values": {} }))).code,
        code::INVALID_PARAMS
    );
    assert_eq!(err(set_many(&mut w, &json!({}))).code, code::INVALID_PARAMS);
}

#[test]
fn a_profile_survives_export_and_import_and_diff_lists_the_differences() {
    let mut a = world("profile-a");
    ok(set(&mut a, &json!({ "key": "look", "value": "natural" })));
    ok(set(&mut a, &json!({ "key": "ui_scale", "value": 130 })));
    let d = ok(diff(&mut a, &json!({})));
    let keys: Vec<&str> = d["differences"]
        .as_array()
        .expect("rows")
        .iter()
        .filter_map(|r| r["key"].as_str())
        .collect();
    assert_eq!(keys.len(), d["count"].as_u64().unwrap_or(0) as usize);
    assert!(
        keys.contains(&"look") && keys.contains(&"ui_scale"),
        "{keys:?}"
    );
    let row = d["differences"]
        .as_array()
        .and_then(|rows| rows.iter().find(|r| r["key"] == "look"))
        .expect("look row");
    assert_eq!(
        (row["value"].as_str(), row["default"].as_str()),
        (Some("natural"), Some("vivid"))
    );

    let profile = ok(export(&mut a, &json!({})));
    assert_eq!(profile["format"], "opts-settings");
    assert_eq!(profile["settings"]["look"], "natural");
    assert!(
        profile["settings"].get("high_contrast").is_none(),
        "defaults are left out"
    );
    let everything = ok(export(&mut a, &json!({ "all": true })));
    assert!(everything["count"].as_u64() > profile["count"].as_u64());

    // Into a fresh world: the profile applies; settings it does not name stay as they are,
    // unless `reset_missing` is asked for.
    let mut b = world("profile-b");
    ok(set(
        &mut b,
        &json!({ "key": "high_contrast", "value": true }),
    ));
    ok(import(&mut b, &profile));
    assert_eq!(value_of(&mut b, "look"), "natural");
    assert_eq!(value_of(&mut b, "ui_scale"), "130");
    assert_eq!(
        value_of(&mut b, "high_contrast"),
        "true",
        "not in the profile: untouched"
    );
    ok(import(
        &mut b,
        &json!({ "settings": profile["settings"].clone(), "reset_missing": true }),
    ));
    assert_eq!(
        value_of(&mut b, "high_contrast"),
        "false",
        "reset to match the profile"
    );
    let after = ok(export(&mut b, &json!({})));
    assert_eq!(after["settings"], profile["settings"]);
}
