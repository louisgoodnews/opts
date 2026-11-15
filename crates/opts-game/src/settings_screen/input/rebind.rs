//! The key-binding capture: the next key press of a waiting binding is stored through the registry.

use bevy::prelude::*;

use crate::settings_registry::{ChangeSource, SettingsApi};
use crate::settings_screen::page::SettingsPageState;

/// While a key binding waits for its key: the next key press (with the modifiers held) is
/// stored through the registry, so a conflict is refused with its reason on the page.
pub fn capture_rebind(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<SettingsPageState>,
    mut api: SettingsApi,
    loc: Res<opts_gui::l10n::Localization>,
) {
    let Some(setting) = state.rebinding else {
        return;
    };
    const MODIFIERS: [KeyCode; 8] = [
        KeyCode::ShiftLeft,
        KeyCode::ShiftRight,
        KeyCode::ControlLeft,
        KeyCode::ControlRight,
        KeyCode::AltLeft,
        KeyCode::AltRight,
        KeyCode::SuperLeft,
        KeyCode::SuperRight,
    ];
    let Some(key) = keys
        .get_just_pressed()
        .find(|k| !MODIFIERS.contains(k))
        .copied()
    else {
        return;
    };
    state.rebinding = None;
    state.dirty = true;
    if key == KeyCode::Escape {
        state.notice = None;
        return;
    }
    let chord = opts_bevy::KeyChord {
        key,
        shift: keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]),
        ctrl: keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]),
    };
    let Some(text) = chord.to_text() else {
        state.notice = Some(loc.t("settings-rebind-unnamed"));
        return;
    };
    match api.change(setting, Some(&text), ChangeSource::Page, true) {
        Ok(_) => state.notice = None,
        Err(e) => state.notice = Some(e.to_string()),
    }
}
