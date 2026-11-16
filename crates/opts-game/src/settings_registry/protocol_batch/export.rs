//! The export of the settings and their differences from the defaults (`settings.export`,
//! `settings.diff`).

use bevy::prelude::*;
use opts_bevy::control::params::opt_bool;
use opts_bevy::control::state::Outcome;
use serde_json::{json, Map, Value};

use super::{normalize, SettingKind, SettingsRegistry};
use crate::settings::GameSettings;

/// The settings that differ from their defaults: `(descriptor key, value, default)` of every
/// visible setting that stores a value, in registry order.
pub(super) fn differences(world: &World, all: bool) -> Vec<(&'static str, String, String)> {
    let registry = world.resource::<SettingsRegistry>();
    let settings = world.resource::<GameSettings>();
    registry
        .all()
        .into_iter()
        .filter(|d| !d.hidden && d.is_stored() && !matches!(d.kind, SettingKind::Action))
        .filter_map(|d| {
            let value = registry.text(settings, d.key)?;
            let default = normalize(d, d.default).unwrap_or_else(|_| d.default.to_string());
            (all || value != default).then_some((d.key, value, default))
        })
        .collect()
}

/// `settings.export { all?: bool }`: the settings that differ from the defaults (or all of them)
/// as a profile that `settings.import` takes back.
pub(crate) fn export(world: &mut World, params: &Value) -> Outcome {
    let all = match opt_bool(params, "all") {
        Ok(a) => a.unwrap_or(false),
        Err(e) => return Outcome::err(e),
    };
    let settings: Map<String, Value> = differences(world, all)
        .into_iter()
        .map(|(key, value, _)| (key.to_string(), Value::String(value)))
        .collect();
    Outcome::ok(json!({
        "format": "opts-settings",
        "version": 1,
        "count": settings.len(),
        "settings": settings,
    }))
}

/// `settings.diff`: the settings that differ from the defaults, with both values.
pub(crate) fn diff(world: &mut World, _: &Value) -> Outcome {
    let rows: Vec<Value> = differences(world, false)
        .into_iter()
        .map(|(key, value, default)| json!({ "key": key, "value": value, "default": default }))
        .collect();
    Outcome::ok(json!({ "count": rows.len(), "differences": rows }))
}
