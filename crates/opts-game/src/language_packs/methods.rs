//! The control methods of the language packs: `l10n.languages`, `l10n.set`, `l10n.reload` and
//! `l10n.report`.


use bevy::prelude::*;
use opts_bevy::control::params::{opt_str, str_param};
use opts_bevy::control::state::{ControlRegistry, Outcome};
use opts_cli::protocol::{code, RpcError};
use opts_gui::l10n::Localization;
use serde_json::{json, Value};

use crate::settings_registry::{apply_change, ChangeSource, SettingError};

use super::scan::scan;
use super::LANGUAGE_KEY;
use super::status::{info_json, languages_value};

fn list(world: &mut World, _params: &Value) -> Outcome {
    Outcome::ok(languages_value(world))
}

fn set(world: &mut World, params: &Value) -> Outcome {
    let tag = match str_param(params, "code") {
        Ok(c) => c.trim().to_string(),
        Err(e) => return Outcome::err(e),
    };
    let (known, selectable) = {
        let loc = world.resource::<Localization>();
        let l = loc.languages();
        (
            l.iter().any(|x| x.code == tag) || loc.can_select(&tag),
            loc.can_select(&tag),
        )
    };
    if !known {
        let codes: Vec<String> = world
            .resource::<Localization>()
            .languages()
            .into_iter()
            .map(|l| l.code)
            .collect();
        return Outcome::err(RpcError::new(
            code::NOT_FOUND,
            format!("unknown language `{tag}` (installed: {})", codes.join(", ")),
        ));
    }
    if !selectable {
        return Outcome::err(RpcError::new(
            code::UNSUPPORTED,
            format!("`{tag}` is written right to left, which the interface does not support yet"),
        ));
    }
    let previous = world.resource::<Localization>().locale().to_string();
    match apply_change(world, LANGUAGE_KEY, Some(&tag), ChangeSource::Protocol) {
        Ok(_) => {
            world.resource_mut::<Localization>().set_locale(&tag);
            Outcome::ok(json!({ "current": tag, "previous": previous }))
        }
        Err(SettingError::Invalid(m) | SettingError::Unavailable(m)) => {
            Outcome::err(RpcError::invalid_params(m))
        }
        Err(e) => Outcome::err(RpcError::new(code::FAILED, e.to_string())),
    }
}

fn reload(world: &mut World, _params: &Value) -> Outcome {
    let summary = {
        let mut loc = world.resource_mut::<Localization>();
        scan(&mut loc)
    };
    let mut v = languages_value(world);
    v["usable"] = json!(summary.usable);
    v["broken"] = json!(summary.broken);
    v["errors"] = json!(summary.errors);
    Outcome::ok(v)
}

fn report(world: &mut World, params: &Value) -> Outcome {
    let loc = world.resource::<Localization>();
    let tag = match opt_str(params, "code") {
        Ok(c) => c.map_or_else(|| loc.locale().to_string(), str::to_string),
        Err(e) => return Outcome::err(e),
    };
    let Some(r) = loc.language_report(&tag) else {
        return Outcome::err(RpcError::new(
            code::NOT_FOUND,
            format!("no language `{tag}`; `l10n.languages` lists them"),
        ));
    };
    let current = loc.locale().to_string();
    Outcome::ok(json!({
        "language": info_json(&r.info, &current),
        "missing_keys": r.missing_keys,
        "copied_keys": r.copied_keys,
        "issues": r.issues.iter().map(opts_l10n::Issue::to_json).collect::<Vec<_>>(),
        "issue_lines": r.issues.iter().map(ToString::to_string).collect::<Vec<_>>(),
    }))
}

/// Registers the control methods.
pub fn register(registry: &mut ControlRegistry) {
    registry.register("l10n.languages", list);
    registry.register("l10n.set", set);
    registry.register("l10n.reload", reload);
    registry.register("l10n.report", report);
}
