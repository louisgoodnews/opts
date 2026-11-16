//! Turning protocol parameters into setting changes: values, confirm flags and the change itself.

use super::*;

/// A JSON value as the text a setting stores (numbers and booleans are accepted too).
pub(in crate::settings_registry) fn text_of(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Bool(b) => Some(b.to_string()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

/// The parameter `value` as stored text.
pub(in crate::settings_registry) fn value_text(params: &Value) -> Result<String, RpcError> {
    match params.get("value") {
        Some(v) => text_of(v).ok_or_else(|| {
            RpcError::invalid_params("parameter `value` must be a string, number or boolean")
        }),
        None => Err(RpcError::invalid_params("missing parameter `value`")),
    }
}

/// The optional parameter `confirm` (default true: apply and save at once).
pub(in crate::settings_registry) fn confirm_param(params: &Value) -> Result<bool, RpcError> {
    opt_bool(params, "confirm").map(|c| c.unwrap_or(true))
}

/// Applies a change and answers with the setting, its previous value, whether it changed
/// anything (`applied`) and whether it takes effect after a restart (`needs_restart`). With
/// `confirm` false a setting that has a revert prompt on the page is applied but not saved and
/// waits for `settings.confirm`, `settings.revert` or the timeout (`pending_confirm`).
pub(in crate::settings_registry) fn changed(
    world: &mut World,
    key: &str,
    value: Option<&str>,
    confirm: bool,
) -> Outcome {
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
