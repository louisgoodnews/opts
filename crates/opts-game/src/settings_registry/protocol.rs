//! Control-protocol methods `settings.list`, `settings.get`, `settings.set`, `settings.reset`
//! (`docs/src/architecture/control-protocol.md`), and the registration of the confirm and revert
//! ([`super::protocol_confirm`]) and batch ([`super::protocol_batch`]) methods. They use the same
//! validation and the same effects as the settings page.

use bevy::prelude::*;
use opts_bevy::control::params::{opt_bool, opt_str, str_param};
use opts_bevy::control::state::{ControlRegistry, Outcome};
use opts_cli::protocol::{code, RpcError};
use serde_json::{json, Value};

use super::{
    apply_change_with, protocol_batch, protocol_confirm, Apply, Available, ChangeSource,
    SettingChanged, SettingDescriptor, SettingError, SettingKind, SettingsRegistry,
};
use crate::settings::GameSettings;

/// Registers the methods when the control server exists.
pub fn register(app: &mut App) {
    let Some(mut registry) = app.world_mut().get_resource_mut::<ControlRegistry>() else {
        return;
    };
    registry.register("settings.list", list);
    registry.register("settings.get", get);
    registry.register("settings.set", set);
    registry.register("settings.reset", reset);
    registry.register("settings.reset_keys", reset_keys);
    registry.register("settings.confirm", protocol_confirm::confirm);
    registry.register("settings.revert", protocol_confirm::revert);
    registry.register("settings.set_many", protocol_batch::set_many);
    registry.register("settings.import", protocol_batch::import);
    registry.register("settings.export", protocol_batch::export);
    registry.register("settings.diff", protocol_batch::diff);
}

/// The protocol code and text of a refused change.
pub(super) fn code_of(e: &SettingError) -> (i64, String) {
    match e {
        SettingError::Unknown(_) => (code::NOT_FOUND, e.to_string()),
        SettingError::Invalid(_) => (code::INVALID_PARAMS, e.to_string()),
        SettingError::Unavailable(_) => (code::NOT_READY, e.to_string()),
    }
}

fn error(e: SettingError) -> Outcome {
    let (c, m) = code_of(&e);
    Outcome::err(RpcError::new(c, m))
}

fn kind_name(k: &SettingKind) -> &'static str {
    match k {
        SettingKind::Bool => "bool",
        SettingKind::Enum(_) => "enum",
        SettingKind::Int { .. } => "int",
        SettingKind::Float { .. } => "float",
        SettingKind::Text => "text",
        SettingKind::Path => "path",
        SettingKind::Time => "time",
        SettingKind::Binding => "binding",
        SettingKind::Action => "action",
    }
}

fn describe(d: &SettingDescriptor, value: Option<String>) -> Value {
    let options: Vec<&str> = match d.kind {
        SettingKind::Enum(o) => o.iter().map(|o| o.value).collect(),
        _ => Vec::new(),
    };
    let apply = match d.apply {
        Apply::Live => json!("live"),
        Apply::OnRestart => json!("restart"),
        Apply::WithRevert(s) => json!({ "revert_after_s": s }),
    };
    let available = match d.available {
        Available::Always => json!("yes"),
        Available::Capability(c) => json!({ "needs": c }),
        Available::NotYet(p) => json!({ "not_yet": p.milestone }),
    };
    json!({
        "key": d.key,
        "category": d.category.id(),
        "kind": kind_name(&d.kind),
        "value": value,
        "default": d.default,
        "options": options,
        "apply": apply,
        "available": available,
        "hidden": d.hidden,
    })
}

fn one(world: &World, key: &str) -> Result<Value, SettingError> {
    let registry = world.resource::<SettingsRegistry>();
    let d = registry
        .get(key)
        .ok_or_else(|| SettingError::Unknown(key.to_string()))?;
    let value = registry.text(world.resource::<GameSettings>(), key);
    Ok(describe(d, value.filter(|_| d.is_stored())))
}

pub(crate) fn list(world: &mut World, params: &Value) -> Outcome {
    let cat = match opt_str(params, "category") {
        Ok(c) => c,
        Err(e) => return Outcome::err(e),
    };
    let filter = match cat.map(|c| (c, super::Category::parse(c))) {
        Some((c, None)) => {
            return Outcome::err(RpcError::invalid_params(format!(
                "unknown category `{c}` (graphics, audio, controls, interface, gameplay, content, advanced)"
            )))
        }
        Some((_, Some(c))) => Some(c),
        None => None,
    };
    let keys: Vec<&'static str> = world
        .resource::<SettingsRegistry>()
        .all()
        .into_iter()
        .filter(|d| filter.is_none_or(|c| d.category == c))
        .map(|d| d.key)
        .collect();
    let items: Vec<Value> = keys.iter().filter_map(|k| one(world, k).ok()).collect();
    Outcome::ok(json!({ "count": items.len(), "settings": items }))
}

pub(crate) fn get(world: &mut World, params: &Value) -> Outcome {
    match str_param(params, "key") {
        Ok(k) => one(world, k).map_or_else(error, Outcome::ok),
        Err(e) => Outcome::err(e),
    }
}

/// A JSON value as the text a setting stores (numbers and booleans are accepted too).
pub(super) fn text_of(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Bool(b) => Some(b.to_string()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

/// The parameter `value` as stored text.
fn value_text(params: &Value) -> Result<String, RpcError> {
    match params.get("value") {
        Some(v) => text_of(v).ok_or_else(|| {
            RpcError::invalid_params("parameter `value` must be a string, number or boolean")
        }),
        None => Err(RpcError::invalid_params("missing parameter `value`")),
    }
}

/// The optional parameter `confirm` (default true: apply and save at once).
fn confirm_param(params: &Value) -> Result<bool, RpcError> {
    opt_bool(params, "confirm").map(|c| c.unwrap_or(true))
}

/// Applies a change and answers with the setting, its previous value, whether it changed
/// anything (`applied`) and whether it takes effect after a restart (`needs_restart`). With
/// `confirm` false a setting that has a revert prompt on the page is applied but not saved and
/// waits for `settings.confirm`, `settings.revert` or the timeout (`pending_confirm`).
fn changed(world: &mut World, key: &str, value: Option<&str>, confirm: bool) -> Outcome {
    let apply = world
        .resource::<SettingsRegistry>()
        .get(key)
        .map(|d| d.apply);
    let hold = match apply {
        Some(Apply::WithRevert(s)) if !confirm => Some(s),
        _ => None,
    };
    if hold.is_some() && protocol_confirm::is_pending(world) {
        return Outcome::err(RpcError::new(
            code::BUSY,
            "another risky setting change waits for confirmation: settings.confirm or settings.revert it first",
        ));
    }
    let c = match apply_change_with(world, key, value, ChangeSource::Protocol, hold.is_none()) {
        Ok(c) => c,
        Err(e) => return error(e),
    };
    let did = c.old != c.new;
    if let Some(seconds) = hold.filter(|_| did) {
        protocol_confirm::begin(world, key, &c.old, &c.new, seconds);
    }
    match one(world, key) {
        Ok(mut v) => {
            v["previous"] = json!(c.old);
            v["applied"] = json!(did);
            v["needs_restart"] = json!(did && matches!(apply, Some(Apply::OnRestart)));
            if let Some(seconds) = hold.filter(|_| did) {
                v["pending_confirm"] = json!(true);
                v["confirm_within_s"] = json!(seconds);
            }
            Outcome::ok(v)
        }
        Err(e) => error(e),
    }
}

pub(crate) fn set(world: &mut World, params: &Value) -> Outcome {
    let key = match str_param(params, "key") {
        Ok(k) => k.to_string(),
        Err(e) => return Outcome::err(e),
    };
    match (value_text(params), confirm_param(params)) {
        (Ok(v), Ok(confirm)) => changed(world, &key, Some(&v), confirm),
        (Err(e), _) | (_, Err(e)) => Outcome::err(e),
    }
}

pub(crate) fn reset(world: &mut World, params: &Value) -> Outcome {
    match str_param(params, "key") {
        Ok(k) => {
            let key = k.to_string();
            match confirm_param(params) {
                Ok(confirm) => changed(world, &key, None, confirm),
                Err(e) => Outcome::err(e),
            }
        }
        Err(e) => Outcome::err(e),
    }
}

/// `settings.reset_keys {group?}`: every key binding (or those of `group`: `vehicle`, `drive`,
/// `camera`, `map`, `ui`) back to its default, saved once.
pub(crate) fn reset_keys(world: &mut World, params: &Value) -> Outcome {
    let group = match opt_str(params, "group") {
        Ok(g) => g
            .filter(|g| !g.eq_ignore_ascii_case("all"))
            .map(str::to_lowercase),
        Err(e) => return Outcome::err(e),
    };
    if let Some(g) = &group {
        if !crate::action_settings::GROUPS.contains(&g.as_str()) {
            return Outcome::err(RpcError::invalid_params(format!(
                "unknown group `{g}` (all, {})",
                crate::action_settings::GROUPS.join(", ")
            )));
        }
    }
    let registry = world.resource::<SettingsRegistry>().clone();
    let changed = crate::action_settings::reset_bindings(
        &registry,
        &mut world.resource_mut::<GameSettings>(),
        group.as_deref(),
    );
    for (key, old, new) in &changed {
        world.write_message(SettingChanged {
            key: key.clone(),
            old: old.clone(),
            new: new.clone(),
            source: ChangeSource::Protocol,
        });
    }
    if !changed.is_empty() {
        world.resource_mut::<GameSettings>().save_in_background();
    }
    let list: Vec<Value> = changed
        .iter()
        .map(|(k, old, new)| json!({ "key": k, "previous": old, "value": new }))
        .collect();
    Outcome::ok(json!({ "changed": list.len(), "bindings": list }))
}

#[cfg(test)]
mod tests {
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
}

// The registry-driven coverage and the confirm, batch and profile tests (M-0072); a file named
// `tests.rs` is what the no-blocking-calls lint of the UI modules treats as test code.
#[cfg(test)]
#[path = "protocol/tests.rs"]
mod coverage;
