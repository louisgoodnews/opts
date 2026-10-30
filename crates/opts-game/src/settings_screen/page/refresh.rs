//! Dirty-tracking and in-place row refresh systems for the generic settings page.

use super::*;

/// Marks the page dirty when something it cannot update in place changed: the language, or a
/// path or free text value.
pub fn mark_dirty(
    mut state: ResMut<SettingsPageState>,
    mut changes: MessageReader<SettingChanged>,
    watched: super::params::DirtyFacts,
    added: Query<(), Added<SettingsPage>>,
    mut themes: Local<Vec<String>>,
) {
    let super::params::DirtyFacts {
        registry,
        loc,
        facts,
        light,
        fonts,
    } = watched;
    if fonts.is_some_and(|f| f.is_changed()) {
        state.dirty = true;
    }
    if light.is_some_and(|l| l.is_changed()) {
        // The values the automatic mode shows (and which controls are locked) changed.
        state.dirty = true;
    }
    if themes.as_slice() != facts.theme_files() {
        *themes = facts.theme_files().to_vec();
        state.dirty = true;
    }
    let structural = changes.read().any(|c| {
        c.key == crate::settings_registry::lighting::MODE_KEY
            || registry.get(&c.key).is_some_and(|d| {
                matches!(d.kind, SettingKind::Path)
                    || (matches!(d.kind, SettingKind::Text) && d.key != "theme_file")
            })
    });
    if structural || loc.is_changed() || !added.is_empty() {
        state.dirty = true;
    }
}

/// Shows the new value of a changed setting in its row without rebuilding it (keeps focus).
pub fn refresh_rows(
    mut commands: Commands,
    mut changes: MessageReader<SettingChanged>,
    facts: super::params::RowFacts,
    nodes: super::params::RowNodes,
) {
    let super::params::RowFacts {
        registry,
        settings,
        loc,
        theme,
        hardware,
    } = facts;
    let super::params::RowNodes {
        mut options,
        mut labels,
        mut switches,
        mut values,
        resets,
        mut now,
    } = nodes;
    let keys: Vec<String> = changes.read().map(|c| c.key.clone()).collect();
    let text = |k: &str| registry.text(&settings, k).unwrap_or_default();
    for (op, mut bg, mut border, kids) in &mut options {
        let SettingOp::Set { key, value } = op else {
            continue;
        };
        if !keys.iter().any(|k| k == key) {
            continue;
        }
        let (b, f, o) = if text(key) == *value {
            ROLES_SELECTED
        } else {
            ROLES_PLAIN
        };
        bg.0 = b;
        border.0 = o;
        for k in kids.into_iter().flatten() {
            if let Ok(mut fg) = labels.get_mut(*k) {
                fg.0 = f;
            }
        }
    }
    for (s, mut sw) in &mut switches {
        if keys.iter().any(|k| k == s.0) {
            sw.is_on = text(s.0) == "true";
        }
    }
    for (v, mut t) in &mut values {
        if keys.iter().any(|k| k == v.0) {
            let profile = hardware.as_ref().and_then(|h| h.profile.as_ref());
            t.0 = super::super::rows::shown_value(&loc, profile, v.0, &text(v.0));
        }
    }
    for (e, op, disabled) in &resets {
        let SettingOp::Reset { key } = op else {
            continue;
        };
        if !keys.iter().any(|k| k == key) {
            continue;
        }
        let at_default = registry.get(key).is_some_and(|d| d.default == text(key));
        sync_reset(&mut commands, e, at_default, disabled, &loc);
    }
    if theme.is_changed() || !keys.is_empty() {
        for mut args in &mut now {
            *args = L10nArgs::one("theme", loc.t(theme_name_key(theme.kind())));
        }
    }
}

/// Disables a reset button whose setting is at its default, and enables it again otherwise.
fn sync_reset(
    commands: &mut Commands,
    e: Entity,
    at_default: bool,
    disabled: bool,
    loc: &Localization,
) {
    if at_default && !disabled {
        commands.entity(e).insert((
            opts_gui::interaction::state::Disabled,
            opts_gui::interaction::state::DisabledReason(loc.t("settings-reset-disabled").into()),
        ));
    } else if !at_default && disabled {
        commands.entity(e).remove::<(
            opts_gui::interaction::state::Disabled,
            opts_gui::interaction::state::DisabledReason,
        )>();
    }
}
