//! The protocol methods that read and write settings: list, get, set, reset and reset_keys.

use super::*;

pub(crate) fn list(world: &mut World, params: &Value) -> Outcome {
    let cat = match opt_str(params, "category") {
        Ok(c) => c,
        Err(e) => return Outcome::err(e),
    };
    let filter = match cat.map(|c| (c, super::super::Category::parse(c))) {
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
