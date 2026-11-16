//! The JSON of the languages and of their problems, for the control methods.

use bevy::prelude::*;
use opts_gui::l10n::LanguageInfo;
use opts_gui::l10n::Localization;
use serde_json::{json, Value};

use super::packs_dir;

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

pub(super) fn problems_json(loc: &Localization) -> Value {
    Value::Array(
        loc.pack_problems()
            .into_iter()
            .map(|(folder, issues)| {
                json!({
                    "folder": folder,
                    "issues": issues.iter().map(opts_l10n::Issue::to_json).collect::<Vec<_>>(),
                })
            })
            .collect(),
    )
}

pub(super) fn languages_value(world: &World) -> Value {
    let loc = world.resource::<Localization>();
    let current = loc.locale().to_string();
    let list: Vec<Value> = loc
        .languages()
        .iter()
        .map(|l| info_json(l, &current))
        .collect();
    json!({
        "current": current,
        "packs_dir": packs_dir().map(|d| d.display().to_string()),
        "languages": list,
        "problems": problems_json(loc),
    })
}
