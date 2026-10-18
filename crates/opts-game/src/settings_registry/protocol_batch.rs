//! Batch methods of the settings protocol (M-0072): `settings.set_many` (atomic),
//! `settings.import`, `settings.export` and `settings.diff` against the defaults.
//!
//! A batch is validated as a whole first, on a scratch copy of the settings: if one entry is
//! refused (unknown key, unavailable setting, bad value, a key binding that clashes with another
//! in the same batch) nothing is applied and the answer lists every refusal. Risky settings are
//! applied at once, without the confirmation of `settings.set confirm:false`.

use bevy::prelude::*;
use opts_bevy::control::params::opt_bool;
use opts_bevy::control::state::Outcome;
use opts_cli::protocol::RpcError;
use serde_json::{json, Map, Value};

use super::protocol::{code_of, text_of};
use super::{
    apply_change_with, normalize, Apply, ChangeSource, SettingKind, SettingsCapabilities,
    SettingsRegistry,
};
use crate::settings::GameSettings;

/// Most entries one batch accepts.
const MAX_ENTRIES: usize = 1000;

/// The `(key, stored text)` entries of the object parameter `field`, in key order.
fn entries(params: &Value, field: &str) -> Result<Vec<(String, String)>, RpcError> {
    let Some(object) = params.get(field).and_then(Value::as_object) else {
        return Err(RpcError::invalid_params(format!(
            "parameter `{field}` must be an object of setting key to value"
        )));
    };
    if object.len() > MAX_ENTRIES {
        return Err(RpcError::invalid_params(format!(
            "at most {MAX_ENTRIES} settings per call"
        )));
    }
    let mut out = Vec::with_capacity(object.len());
    for (key, value) in object {
        let text = text_of(value).ok_or_else(|| {
            RpcError::invalid_params(format!(
                "`{key}`: value must be a string, number or boolean"
            ))
        })?;
        out.push((key.clone(), text));
    }
    out.sort();
    Ok(out)
}

/// What the batch would be refused for, found on a scratch copy of the settings.
fn refusals(world: &World, batch: &[(String, String)]) -> Vec<Value> {
    let registry = world.resource::<SettingsRegistry>();
    let caps = world
        .get_resource::<SettingsCapabilities>()
        .cloned()
        .unwrap_or_default();
    let mut scratch = world.resource::<GameSettings>().clone();
    batch
        .iter()
        .filter_map(|(key, text)| {
            let e = registry.set(&mut scratch, &caps, key, text).err()?;
            let (code, message) = code_of(&e);
            Some(json!({ "key": key, "code": code, "message": message }))
        })
        .collect()
}

/// Applies a validated batch and answers with what changed.
fn apply(world: &mut World, batch: &[(String, String)]) -> Outcome {
    let bad = refusals(world, batch);
    if let Some(first) = bad.first() {
        let text = format!(
            "{} of {} settings refused, nothing changed; first: {}",
            bad.len(),
            batch.len(),
            first["message"].as_str().unwrap_or_default()
        );
        return Outcome::err(RpcError::invalid_params(text).with_data(json!({ "errors": bad })));
    }
    let (mut changed, mut restart, mut unchanged) = (Vec::new(), Vec::new(), 0);
    for (key, text) in batch {
        let apply = world
            .resource::<SettingsRegistry>()
            .get(key)
            .map(|d| d.apply);
        match apply_change_with(world, key, Some(text), ChangeSource::Protocol, false) {
            Ok(c) if c.old != c.new => {
                let needs_restart = matches!(apply, Some(Apply::OnRestart));
                if needs_restart {
                    restart.push(key.clone());
                }
                changed.push(json!({
                    "key": key, "previous": c.old, "value": c.new, "needs_restart": needs_restart,
                }));
            }
            Ok(_) => unchanged += 1,
            // Cannot happen after the dry run; reported, not hidden.
            Err(e) => {
                tracing::warn!(err = %e, key, "batch entry refused after validation");
                return Outcome::err(RpcError::new(
                    opts_cli::protocol::code::INTERNAL_ERROR,
                    e.to_string(),
                ));
            }
        }
    }
    if !changed.is_empty() {
        world.resource_mut::<GameSettings>().save_in_background();
    }
    Outcome::ok(json!({
        "count": batch.len(),
        "changed": changed,
        "unchanged": unchanged,
        "needs_restart": restart,
    }))
}

/// `settings.set_many { values: {key: value, ...} }`: all or nothing.
pub(crate) fn set_many(world: &mut World, params: &Value) -> Outcome {
    match entries(params, "values") {
        Ok(batch) if batch.is_empty() => Outcome::err(RpcError::invalid_params(
            "parameter `values` has no settings",
        )),
        Ok(batch) => apply(world, &batch),
        Err(e) => Outcome::err(e),
    }
}

/// The settings that differ from their defaults: `(descriptor key, value, default)` of every
/// visible setting that stores a value, in registry order.
fn differences(world: &World, all: bool) -> Vec<(&'static str, String, String)> {
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

/// `settings.import { values | settings, reset_missing?: bool }`: applies a profile like
/// `settings.set_many`; with `reset_missing` the settings that differ from the defaults and are
/// not in the profile go back to their defaults, so the result is exactly the profile.
pub(crate) fn import(world: &mut World, params: &Value) -> Outcome {
    let field = if params.get("settings").is_some() {
        "settings"
    } else {
        "values"
    };
    let mut batch = match entries(params, field) {
        Ok(b) => b,
        Err(e) => return Outcome::err(e),
    };
    let reset_missing = match opt_bool(params, "reset_missing") {
        Ok(r) => r.unwrap_or(false),
        Err(e) => return Outcome::err(e),
    };
    if reset_missing {
        let listed: std::collections::HashSet<String> =
            batch.iter().map(|(k, _)| k.clone()).collect();
        batch.extend(
            differences(world, false)
                .into_iter()
                .filter(|(key, _, _)| !listed.contains(*key))
                .map(|(key, _, default)| (key.to_string(), default)),
        );
        batch.sort();
    }
    apply(world, &batch)
}
