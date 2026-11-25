//! The keys held in the settings: the reset of the bindings, the chord of an action, and the
//! sync of [`ActionKeys`] and of the mouse driving mode with the settings.

use bevy::prelude::*;
use opts_bevy::action_keys::{self, ActionDef, ActionKeys};
use opts_bevy::gamepad_input::PadBindings;
use opts_bevy::KeyChord;

use crate::settings::GameSettings;
use crate::settings_registry::SettingsRegistry;

use super::pad::sync_pad_bindings;
use super::{binding_settings, MOUSE_DRIVING_KEY};

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

/// Keeps [`opts_bevy::MouseDriving::enabled`] equal to the setting.
pub fn sync_mouse_driving(settings: Res<GameSettings>, mut mouse: ResMut<opts_bevy::MouseDriving>) {
    if !(settings.is_changed() || mouse.is_added()) {
        return;
    }
    let enabled = settings.raw(MOUSE_DRIVING_KEY).as_deref() != Some("false");
    if mouse.enabled != enabled {
        mouse.enabled = enabled;
    }
}

/// Registers the resource and its sync.
pub struct ActionKeysPlugin;

impl Plugin for ActionKeysPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ActionKeys::with_system_layout())
            .init_resource::<opts_bevy::MouseDriving>()
            .init_resource::<PadBindings>()
            .add_systems(
                PreUpdate,
                (
                    sync_action_keys,
                    sync_mouse_driving,
                    sync_pad_bindings,
                    opts_bevy::action_keys::learn_key_layout,
                ),
            );
    }
}
