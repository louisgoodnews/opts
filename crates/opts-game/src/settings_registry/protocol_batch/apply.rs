//! Applying a batch of settings (`settings.set_many`, `settings.import`): validated as a whole
//! on a scratch copy first, then applied and answered with what changed.

use bevy::prelude::*;
use opts_bevy::control::params::opt_bool;
use opts_bevy::control::state::Outcome;
use opts_cli::protocol::RpcError;
use serde_json::{json, Value};

use super::{apply_change_with, Apply, ChangeSource, SettingsCapabilities, SettingsRegistry};
use crate::settings::GameSettings;
use crate::settings_registry::protocol::code_of;

use super::entries;
use super::export::differences;

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
        crate::settings::save_in_background(&world.resource_mut::<GameSettings>());
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
