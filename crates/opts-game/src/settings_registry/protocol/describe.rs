//! Describing settings for the protocol: the error codes, the kinds and the JSON of a setting.

use super::*;

/// The protocol code and text of a refused change.
pub(in crate::settings_registry) fn code_of(e: &SettingError) -> (i64, String) {
    match e {
        SettingError::Unknown(_) => (code::NOT_FOUND, e.to_string()),
        SettingError::Invalid(_) => (code::INVALID_PARAMS, e.to_string()),
        SettingError::Unavailable(_) => (code::NOT_READY, e.to_string()),
    }
}

pub(in crate::settings_registry) fn error(e: SettingError) -> Outcome {
    let (c, m) = code_of(&e);
    Outcome::err(RpcError::new(c, m))
}

pub(in crate::settings_registry) fn kind_name(k: &SettingKind) -> &'static str {
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

pub(in crate::settings_registry) fn describe(d: &SettingDescriptor, value: Option<String>) -> Value {
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

pub(in crate::settings_registry) fn one(world: &World, key: &str) -> Result<Value, SettingError> {
    let registry = world.resource::<SettingsRegistry>();
    let d = registry
        .get(key)
        .ok_or_else(|| SettingError::Unknown(key.to_string()))?;
    let value = registry.text(world.resource::<GameSettings>(), key);
    Ok(describe(d, value.filter(|_| d.is_stored())))
}
