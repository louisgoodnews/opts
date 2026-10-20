//! The key bindings of the settings (M-0142): the built-in actions of `opts_bevy::action_keys`
//! as `controls.<group>.<id>` rows next to the vehicle actions (`controls.vehicle.<id>`), the
//! conflict check across all of them, the live link to [`opts_bevy::ActionKeys`] and the reset of
//! one group or of every binding.

use bevy::prelude::*;
use opts_bevy::action_keys::{self, ActionDef, ActionKeys};
use opts_bevy::KeyChord;

use crate::settings::GameSettings;
use crate::settings_registry::descriptor::{
    Category as C, SettingDescriptor as D, SettingKind as K,
};
use crate::settings_registry::SettingsRegistry;
use crate::vehicle_keys;

/// The groups of key bindings, in the order of the reset rows.
pub const GROUPS: [&str; 5] = ["vehicle", "drive", "camera", "map", "ui"];

/// The prefix of the reset rows (`controls.reset.map`, `controls.reset.all`).
pub const RESET_PREFIX: &str = "controls.reset.";

const RESET_ROWS: [(&str, &str, &str); 6] = [
    (
        "controls.reset.all",
        "set-reset-keys-all",
        "set-reset-keys-all-desc",
    ),
    (
        "controls.reset.vehicle",
        "set-reset-keys-vehicle",
        "set-reset-keys-vehicle-desc",
    ),
    (
        "controls.reset.drive",
        "set-reset-keys-drive",
        "set-reset-keys-drive-desc",
    ),
    (
        "controls.reset.camera",
        "set-reset-keys-camera",
        "set-reset-keys-camera-desc",
    ),
    (
        "controls.reset.map",
        "set-reset-keys-map",
        "set-reset-keys-map-desc",
    ),
    (
        "controls.reset.ui",
        "set-reset-keys-ui",
        "set-reset-keys-ui-desc",
    ),
];

/// Message key of an action's label and description.
fn label_keys(id: &str) -> (&'static str, &'static str) {
    // Leaked once per action at registration: the registry wants `'static` keys.
    let l: &'static str = Box::leak(format!("set-bk-{id}").into_boxed_str());
    let d: &'static str = Box::leak(format!("set-bk-{id}-desc").into_boxed_str());
    (l, d)
}

fn leak(text: String) -> &'static str {
    Box::leak(text.into_boxed_str())
}

/// The descriptors: one binding row per built-in action and the reset rows.
#[must_use]
pub fn descriptors() -> Vec<D> {
    let mut v: Vec<D> = action_keys::ACTIONS
        .iter()
        .enumerate()
        .map(|(i, a)| {
            let (label, desc) = label_keys(a.id);
            D::new(
                leak(action_keys::setting_of(a)),
                K::Binding,
                a.default,
                C::Controls,
                200 + i as u16,
                label,
                desc,
            )
            .keywords(Box::leak(
                vec!["key", "keyboard", "binding", a.group].into_boxed_slice(),
            ))
        })
        .collect();
    v.extend(
        RESET_ROWS
            .iter()
            .enumerate()
            .map(|(i, (key, label, desc))| {
                D::new(key, K::Action, "", C::Controls, 400 + i as u16, label, desc)
                    .keywords(&["key", "keyboard", "binding", "reset", "default"])
            }),
    );
    v
}

/// Every binding setting with its default and group: the vehicle actions and the built-in ones.
#[must_use]
pub fn binding_settings() -> Vec<(String, &'static str, &'static str)> {
    let mut v: Vec<(String, &'static str, &'static str)> = vehicle_keys::ACTIONS
        .iter()
        .map(|a| (a.setting.to_owned(), a.default, "vehicle"))
        .collect();
    v.extend(
        action_keys::ACTIONS
            .iter()
            .map(|a| (action_keys::setting_of(a), a.default, a.group)),
    );
    v
}

/// Whether `key` is the default key of a built-in action: those keys are no longer fixed, the
/// action that holds the chord is named instead.
fn moved_by_an_action(key: KeyCode) -> bool {
    action_keys::ACTIONS.iter().any(|a| {
        KeyChord::parse(a.default)
            .ok()
            .flatten()
            .is_some_and(|c| c.key == key)
    })
}

/// Why `chord` cannot be the key of `setting`: a key OPTS keeps for itself, or another action's
/// key (vehicle or built-in).
#[must_use]
pub fn conflict(
    registry: &SettingsRegistry,
    settings: &GameSettings,
    setting: &str,
    chord: KeyChord,
) -> Option<String> {
    if chord.is_reserved() && !moved_by_an_action(chord.key) {
        return Some(format!(
            "{} is used by driving, the cameras or the overlays and cannot be bound (Ctrl+ combinations can)",
            chord.to_text().unwrap_or_default()
        ));
    }
    for (other, _, _) in binding_settings().iter().filter(|(s, _, _)| s != setting) {
        let text = registry.text(settings, other).unwrap_or_default();
        if KeyChord::parse(&text).ok().flatten() == Some(chord) {
            return Some(format!(
                "{} is already the key of {}",
                chord.to_text().unwrap_or_default(),
                other.trim_start_matches("controls.")
            ));
        }
    }
    None
}

/// The bindings of `group` (`None`: all) put back to their defaults, without conflict checks (the
/// defaults are unique across all actions, a test holds that). Returns the changes
/// `(setting, old, new)`.
pub fn reset_bindings(
    registry: &SettingsRegistry,
    settings: &mut GameSettings,
    group: Option<&str>,
) -> Vec<(String, String, String)> {
    let mut changed = Vec::new();
    for (setting, default, g) in binding_settings() {
        if group.is_some_and(|want| want != g) {
            continue;
        }
        let old = registry.text(settings, &setting).unwrap_or_default();
        if old != default {
            settings.set_raw(&setting, default);
            changed.push((setting, old, default.to_owned()));
        }
    }
    changed
}

/// The chord that the settings hold for a built-in action.
#[must_use]
pub fn chord_of(
    registry: &SettingsRegistry,
    settings: &GameSettings,
    a: &ActionDef,
) -> Option<KeyChord> {
    let text = registry.text(settings, &action_keys::setting_of(a))?;
    KeyChord::parse(&text).ok().flatten()
}

/// Keeps [`ActionKeys`] equal to the settings.
pub fn sync_action_keys(
    registry: Res<SettingsRegistry>,
    settings: Res<GameSettings>,
    mut keys: ResMut<ActionKeys>,
) {
    if !(settings.is_changed() || keys.is_added()) {
        return;
    }
    let mut want = ActionKeys::default();
    for a in action_keys::ACTIONS {
        want.set(a.id, chord_of(&registry, &settings, a));
    }
    if *keys != want {
        *keys = want;
    }
}

/// Registers the resource and its sync.
pub struct ActionKeysPlugin;

impl Plugin for ActionKeysPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ActionKeys>()
            .add_systems(PreUpdate, sync_action_keys);
    }
}

#[cfg(test)]
mod tests;
