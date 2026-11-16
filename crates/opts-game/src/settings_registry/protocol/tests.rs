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

#[path = "tests/batch.rs"]
mod batch;
#[path = "tests/coverage.rs"]
mod coverage;
#[path = "tests/risky.rs"]
mod risky;
