//! Language packs in the running game: scan at start-up and on demand, the control methods
//! `l10n.languages`, `l10n.set`, `l10n.reload`, `l10n.report`, and the *Reload languages* action
//! of the settings page.
//!
//! The format, the checks and the folder layout are in `opts_core::l10n` and documented in
//! `docs/src/architecture/localisation-packs.md`; the merge with the built-in tables is
//! `opts_gui::l10n::Localization::load_packs`. A broken pack never stops the game: its problems
//! are logged and listed by `l10n.report`, and the English text shows where a message is
//! unusable.

use std::path::PathBuf;

use bevy::prelude::*;
use opts_bevy::control::params::{opt_str, str_param};
use opts_bevy::control::state::{ControlRegistry, Outcome};
use opts_cli::protocol::{code, RpcError};
use opts_core::l10n::check::Severity;
use opts_gui::components::toast::ToastBuilder;
use opts_gui::l10n::LanguageInfo;
use opts_gui::l10n::Localization;
use serde_json::{json, Value};

use crate::settings_registry::{apply_change, ChangeSource, SettingError};
use crate::settings_screen::SettingActionRequested;

/// The settings key of the language.
pub const LANGUAGE_KEY: &str = "interface.language";
/// The settings action that scans the language folder again.
pub const RELOAD_KEY: &str = "interface.language_reload";

/// The folder language packs are read from (`<user data dir>/lang`).
#[must_use]
pub fn packs_dir() -> Option<PathBuf> {
    opts_core::l10n::pack::default_root()
}

/// What a scan found.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ScanSummary {
    /// Packs that can be offered.
    pub usable: usize,
    /// Folders that cannot be used.
    pub broken: usize,
    /// Errors over all packs.
    pub errors: usize,
}

/// Scans the pack folder into `loc`, logs what is wrong, and returns the counts. A locale that
/// has no table after the scan (its pack was removed) falls back to English.
pub fn scan(loc: &mut Localization) -> ScanSummary {
    let Some(dir) = packs_dir() else {
        loc.clear_packs();
        return ScanSummary::default();
    };
    scan_dir(loc, &dir)
}

/// [`scan`] for an explicit folder.
pub fn scan_dir(loc: &mut Localization, dir: &std::path::Path) -> ScanSummary {
    let usable = loc.load_packs(dir);
    let mut summary = ScanSummary {
        usable,
        ..ScanSummary::default()
    };
    for (folder, issues) in loc.pack_problems() {
        summary.broken += 1;
        for i in &issues {
            warn!("language pack `{folder}`: {i}");
        }
    }
    for l in loc.languages().iter().filter(|l| l.from_pack) {
        summary.errors += l.errors;
        if let Some(report) = loc.language_report(&l.code) {
            for i in &report.issues {
                match i.severity {
                    Severity::Error => warn!("language pack `{}`: {i}", l.code),
                    Severity::Warning => debug!("language pack `{}`: {i}", l.code),
                }
            }
        }
    }
    let current = loc.locale().to_string();
    let known = |tag: &str| loc.available().contains(&tag);
    let language = current.split('-').next().unwrap_or(&current).to_string();
    if !known(&current) && !known(&language) {
        info!("language `{current}` is no longer installed; showing English");
        loc.set_locale("en");
    }
    summary
}

/// Start-up: reads the pack folder after every plugin has added its English messages.
pub fn load_startup(mut loc: ResMut<Localization>) {
    let s = scan(&mut loc);
    if s.usable + s.broken > 0 {
        info!(
            usable = s.usable,
            broken = s.broken,
            errors = s.errors,
            "language packs scanned"
        );
    }
}

/// The *Reload languages* button of the settings page.
pub fn reload_action(
    mut requests: MessageReader<SettingActionRequested>,
    mut commands: Commands,
) {
    if requests.read().any(|r| r.key == RELOAD_KEY) {
        commands.queue(|world: &mut World| {
            let summary = {
                let mut loc = world.resource_mut::<Localization>();
                scan(&mut loc)
            };
            let loc = world.resource::<Localization>();
            let mut text = loc.t_args(
                "settings-toast-languages-reloaded",
                &[("count".into(), summary.usable.into())],
            );
            if summary.broken + summary.errors > 0 {
                text.push(' ');
                text.push_str(&loc.t_args(
                    "settings-toast-languages-problems",
                    &[("count".into(), (summary.broken + summary.errors).into())],
                ));
            }
            let mut commands = world.commands();
            ToastBuilder::new(text).with_duration(5.0).spawn(&mut commands);
        });
    }
}

/// JSON of one language for the control methods.
#[must_use]
pub fn info_json(l: &LanguageInfo, current: &str) -> Value {
    json!({
        "code": l.code,
        "name": l.name,
        "native_name": l.native_name,
        "direction": l.direction.name(),
        "builtin": l.builtin,
        "from_pack": l.from_pack,
        "authors": l.authors,
        "version": l.version,
        "total": l.total,
        "translated": l.translated,
        "copied": l.copied,
        "missing": l.missing,
        "percent": (l.percent() * 10.0).round() / 10.0,
        "errors": l.errors,
        "warnings": l.warnings,
        "outdated": l.outdated,
        "missing_fonts": l.missing_fonts,
        "selectable": l.selectable,
        "current": l.code == current,
        "dir": l.dir.as_ref().map(|d| d.display().to_string()),
    })
}

fn problems_json(loc: &Localization) -> Value {
    Value::Array(
        loc.pack_problems()
            .into_iter()
            .map(|(folder, issues)| {
                json!({
                    "folder": folder,
                    "issues": issues.iter().map(opts_core::l10n::Issue::to_json).collect::<Vec<_>>(),
                })
            })
            .collect(),
    )
}

fn languages_value(world: &World) -> Value {
    let loc = world.resource::<Localization>();
    let current = loc.locale().to_string();
    let list: Vec<Value> = loc.languages().iter().map(|l| info_json(l, &current)).collect();
    json!({
        "current": current,
        "packs_dir": packs_dir().map(|d| d.display().to_string()),
        "languages": list,
        "problems": problems_json(loc),
    })
}

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
        "issues": r.issues.iter().map(opts_core::l10n::Issue::to_json).collect::<Vec<_>>(),
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
