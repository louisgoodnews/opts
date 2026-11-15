//! The rebindable vehicle key actions: OMSI's `keyboard.cfg` vehicle triggers that matter for
//! driving a bus or tram (battery, engine start, lights, indicators, hazard lights, doors, horn,
//! interior lights, tickets), each with a setting `controls.vehicle.<id>` that holds the key.
//!
//! The default keys are OMSI's own (`Inputs/keyboard.cfg`, see `docs/src/formats/text/engine-config.md`
//! and `docs/src/simulation/vehicle-keys.md`). The settings are the only source for these actions:
//! the key raises the script trigger on the player's vehicle (`opts_bevy::vehicle_triggers`), and
//! the other `keyboard.cfg` vehicle bindings (IBIS, destination blinds, cash desk) stay as the
//! installation has them. A binding is refused when another action or a key OPTS uses itself has
//! the same chord ([`conflict`]).

use bevy::prelude::*;
use opts_bevy::{KeyChord, ScriptKeyBase, ScriptKeyBinding, ScriptKeyBindings};

use crate::settings::GameSettings;
use crate::settings_registry::{Category, SettingDescriptor, SettingKind, SettingsRegistry};

mod conflict;
mod descriptors;
mod scope;
mod table;
pub use conflict::*;
pub use descriptors::*;
pub use scope::*;
pub use table::*;


/// The action with this id or this setting key.
#[must_use]
pub fn action(id_or_setting: &str) -> Option<&'static VehicleKeyAction> {
    ACTIONS
        .iter()
        .find(|a| a.id == id_or_setting || a.owns(id_or_setting))
}

/// The action that raises `trigger`.
#[must_use]
pub fn action_of_trigger(trigger: &str) -> Option<&'static VehicleKeyAction> {
    ACTIONS.iter().find(|a| a.trigger == trigger)
}
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

/// Registers the sync system.
pub struct VehicleKeysPlugin;

impl Plugin for VehicleKeysPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ScriptKeyBase>()
            .init_resource::<ScriptKeyBindings>()
            .add_systems(PreUpdate, sync_bindings);
    }
}

#[cfg(test)]
mod tests;
