//! The choice of a font family: the family the database does not hold is given back, and the
//! list of the families the user can choose.

use bevy::prelude::*;
use bevy::text::FontCx;
use opts_gui::fonts::{FALLBACK_FAMILY, MONO_FAMILY, UI_FAMILY};

use crate::settings_registry::{ChangeSource, SettingChanged};
use crate::shell::notify::{Durable, NotifyExt};

use super::{FontChoices, SETTING_FONT};

/// Whether the font database holds the family.
fn has_family(cx: &mut FontCx, name: &str) -> bool {
    cx.collection.family_id(name).is_some()
}

/// A family the database does not hold is given back: the previous one stays, an error toast says so.
pub(super) fn refuse_unknown_font(
    mut changes: MessageReader<SettingChanged>,
    font_cx: Option<ResMut<FontCx>>,
    mut commands: Commands,
) {
    let Some(mut cx) = font_cx else {
        for _ in changes.read() {}
        return;
    };
    for c in changes.read() {
        if c.key != SETTING_FONT || c.new.trim().is_empty() || c.source == ChangeSource::Revert {
            continue;
        }
        if has_family(&mut cx, c.new.trim()) {
            continue;
        }
        tracing::warn!(family = %c.new, "the font family is not installed; keeping the previous one");
        let old = c.old.clone();
        // Through the world: this system reads the change messages, the revert writes one.
        commands.queue(move |world: &mut World| {
            if let Err(e) = crate::settings_registry::apply_change(
                world,
                SETTING_FONT,
                Some(&old),
                ChangeSource::Revert,
            ) {
                tracing::warn!("could not restore the font setting: {e}");
            }
        });
        commands.notify_error(
            "settings-font-unavailable",
            vec![("font", c.new.clone())],
            Durable::Inline("settings row ui_font"),
        );
    }
}

/// Keeps [`FontChoices`] in step with the font database when fonts are added.
pub(super) fn list_choices(
    fonts: Option<Res<Assets<Font>>>,
    font_cx: Option<ResMut<FontCx>>,
    mut choices: ResMut<FontChoices>,
    mut seen: Local<usize>,
) {
    let (Some(fonts), Some(mut cx)) = (fonts, font_cx) else {
        return;
    };
    if *seen == fonts.len() && *seen != 0 {
        return;
    }
    *seen = fonts.len();
    let mut names: Vec<String> = cx
        .collection
        .family_names()
        .filter(|n| !n.starts_with("asset_id:") && *n != "Material Icons" && *n != UI_FAMILY)
        .map(str::to_owned)
        .collect();
    // The other bundled faces first, then what the folder added.
    names.sort_by_key(|n| {
        (
            ![FALLBACK_FAMILY, MONO_FAMILY].contains(&n.as_str()),
            n.to_lowercase(),
        )
    });
    names.dedup();
    if choices.0 != names {
        choices.0 = names;
    }
}
