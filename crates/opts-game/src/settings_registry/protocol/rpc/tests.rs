//! Unit tests for the settings control-protocol handlers.

use super::*;
use crate::settings_registry::{SettingChanged, SettingsCapabilities};

fn world(name: &str) -> World {
    let mut w = World::new();
    let dir =
        std::env::temp_dir().join(format!("opts-settings-proto-{}-{name}", std::process::id()));
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

#[test]
fn list_get_set_reset_round_trip() {
    let mut w = world("rt");
    let all = ok(list(&mut w, &json!({})));
    assert!(all["count"].as_u64().unwrap_or(0) >= 20);
    let g = ok(list(&mut w, &json!({ "category": "graphics" })));
    assert!(g["settings"]
        .as_array()
        .is_some_and(|a| a.iter().all(|s| s["category"] == "graphics")));
    assert_eq!(ok(get(&mut w, &json!({ "key": "look" })))["value"], "vivid");
    let s = ok(set(&mut w, &json!({ "key": "look", "value": "natural" })));
    assert_eq!(
        (s["value"].as_str(), s["previous"].as_str()),
        (Some("natural"), Some("vivid"))
    );
    assert_eq!(
        w.resource::<GameSettings>().look,
        crate::look::Look::Natural
    );
    assert_eq!(w.resource::<Messages<SettingChanged>>().len(), 1);
    let r = ok(reset(&mut w, &json!({ "key": "look" })));
    assert_eq!(r["value"], "vivid");
    // numbers and booleans are accepted for numeric and boolean settings
    ok(set(&mut w, &json!({ "key": "ui_scale", "value": 150 })));
    ok(set(
        &mut w,
        &json!({ "key": "high_contrast", "value": true }),
    ));
    assert_eq!(w.resource::<GameSettings>().ui_scale, 150);
    assert!(w.resource::<GameSettings>().high_contrast);
}

#[test]
fn errors_use_the_standard_codes() {
    let mut w = world("err");
    assert_eq!(
        err(get(&mut w, &json!({ "key": "nope" }))).code,
        code::NOT_FOUND
    );
    assert_eq!(
        err(set(&mut w, &json!({ "key": "ui_scale", "value": "999" }))).code,
        code::INVALID_PARAMS
    );
    assert_eq!(
        err(set(
            &mut w,
            &json!({ "key": "gameplay.time_scale", "value": "5" })
        ))
        .code,
        code::NOT_READY
    );
    assert_eq!(
        err(set(&mut w, &json!({ "key": "look" }))).code,
        code::INVALID_PARAMS
    );
    assert_eq!(
        err(list(&mut w, &json!({ "category": "nope" }))).code,
        code::INVALID_PARAMS
    );
    assert_eq!(
        w.resource::<GameSettings>().ui_scale,
        100,
        "nothing changed"
    );
}
