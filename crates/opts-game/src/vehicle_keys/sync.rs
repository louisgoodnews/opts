//! Rebuilds the script key bindings from the keyboard.cfg base and the vehicle key settings.

use bevy::prelude::*;
use opts_bevy::{ScriptKeyBase, ScriptKeyBinding, ScriptKeyBindings};

use crate::settings::GameSettings;
use crate::settings_registry::SettingsRegistry;

use super::{chord_of, ACTIONS};

/// Rebuilds [`ScriptKeyBindings`] from the keyboard.cfg base and the settings whenever either
/// changes.
pub fn sync_bindings(
    registry: Res<SettingsRegistry>,
    settings: Res<GameSettings>,
    base: Res<ScriptKeyBase>,
    mut bindings: ResMut<ScriptKeyBindings>,
    rail: Query<
        (),
        (
            With<opts_bevy::PlayerVehicle>,
            With<crate::in_game::omsi_world::RailPlayer>,
        ),
    >,
    mut first: Local<Option<bool>>,
) {
    let tram = !rail.is_empty();
    if *first == Some(tram) && !settings.is_changed() && !base.is_changed() {
        return;
    }
    *first = Some(tram);
    let own: Vec<ScriptKeyBinding> = ACTIONS
        .iter()
        .filter(|a| a.scope.applies(tram))
        .filter_map(|a| {
            let c = chord_of(&registry, &settings, a, tram)?;
            Some(
                std::iter::once(a.trigger)
                    .chain(a.also)
                    .map(move |t| ScriptKeyBinding {
                        trigger: t.to_string(),
                        key: c.key,
                        shift: c.shift,
                        ctrl: c.ctrl,
                        held: a.held,
                    }),
            )
        })
        .flatten()
        .collect();
    // The triggers of every action are the settings', never the installation's key file's (also
    // those of the actions that do not apply to this vehicle).
    let replaced: Vec<&str> = ACTIONS
        .iter()
        .flat_map(|a| std::iter::once(a.trigger).chain(a.also))
        .collect();
    let composed = ScriptKeyBindings::compose(&base.0, &replaced, own);
    if composed.0 != bindings.0 {
        *bindings = composed;
    }
}
