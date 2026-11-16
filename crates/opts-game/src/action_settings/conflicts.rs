//! The conflict check of a key binding across all bindings, and the scope of a setting.

use bevy::prelude::*;
use opts_bevy::action_keys::{self};
use opts_bevy::KeyChord;

use crate::settings::GameSettings;
use crate::settings_registry::SettingsRegistry;
use crate::vehicle_keys;

use super::binding_settings;

/// The vehicles a binding setting is for.
pub(super) fn scope_of(setting: &str) -> vehicle_keys::Scope {
    if let Some(a) = vehicle_keys::action(setting) {
        // A shared action has a layout per kind of vehicle: its rail key is a tram's, its
        // plain key a bus's.
        return match (a.rail_setting, a.scope) {
            (Some(rail), _) if rail == setting => vehicle_keys::Scope::Tram,
            (Some(_), _) => vehicle_keys::Scope::Bus,
            (None, scope) => scope,
        };
    }
    match action_keys::ACTIONS
        .iter()
        .find(|a| action_keys::setting_of(a) == setting)
        .map(|a| a.scope)
    {
        Some(action_keys::ActionScope::Road) => vehicle_keys::Scope::Bus,
        Some(action_keys::ActionScope::Rail) => vehicle_keys::Scope::Tram,
        _ => vehicle_keys::Scope::Both,
    }
}

/// Whether `key` is the default key of a built-in action that is in use together with a binding of
/// `scope`: such keys are no longer fixed, the action that holds the chord is named instead (W is
/// the throttle of a bus, but the direction selector of a tram).
fn moved_by_an_action(key: KeyCode, scope: vehicle_keys::Scope) -> bool {
    action_keys::ACTIONS.iter().any(|a| {
        let action_scope = match a.scope {
            action_keys::ActionScope::Road => vehicle_keys::Scope::Bus,
            action_keys::ActionScope::Rail => vehicle_keys::Scope::Tram,
            action_keys::ActionScope::All => vehicle_keys::Scope::Both,
        };
        action_scope.overlaps(scope)
            && KeyChord::parse(a.default)
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
    let scope = scope_of(setting);
    if chord.is_reserved() && !moved_by_an_action(chord.key, scope) {
        return Some(format!(
            "{} is used by driving, the cameras or the overlays and cannot be bound (Ctrl+ combinations can)",
            chord.to_text().unwrap_or_default()
        ));
    }
    for (other, _, _) in binding_settings()
        .iter()
        .filter(|(s, _, _)| s != setting && scope_of(s).overlaps(scope))
    {
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
