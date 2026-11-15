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
use opts_bevy::{ScriptKeyBase, ScriptKeyBindings};

mod conflict;
mod descriptors;
mod lookup;
mod sync;
mod table;

pub use conflict::{chord_of, conflict};
pub use descriptors::descriptors;
pub use lookup::{action, action_of_trigger};
pub use sync::sync_bindings;
pub use table::ACTIONS;

/// One rebindable vehicle key action.
#[derive(Debug, Clone, Copy)]
pub struct VehicleKeyAction {
    /// Short name (`lights`), the argument of `vehicle.key`.
    pub id: &'static str,
    /// The setting that holds the key (of buses and trolleybuses when the action is shared).
    pub setting: &'static str,
    /// The setting that holds the key of rail vehicles for an action shared by both kinds
    /// (`controls.rail.<id>`): trams are operated with their own layout, which starts out as the
    /// bus's. `None` for an action of one kind only.
    pub rail_setting: Option<&'static str>,
    /// Message key of the label .
    pub label_key: &'static str,
    /// Message key of the description.
    pub desc_key: &'static str,
    /// The script trigger the key raises.
    pub trigger: &'static str,
    /// OMSI's default key, as setting text.
    pub default: &'static str,
    /// Releasing the key raises `<trigger>_off`.
    pub held: bool,
    /// Which vehicles the key is for: a bus's door keys do nothing on a tram and the tram's
    /// side selection shares its key with the bus's second door.
    pub scope: Scope,
    /// A second trigger the same key raises (vehicles name the same function differently).
    pub also: Option<&'static str>,
}

/// The vehicles a key action is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// Buses, trolleybuses and trams.
    Both,
    /// Buses and trolleybuses.
    Bus,
    /// Trams.
    Tram,
}

impl Scope {
    /// Whether an action of this scope applies to a vehicle (`tram`: it runs on rails).
    #[must_use]
    pub fn applies(self, tram: bool) -> bool {
        match self {
            Self::Both => true,
            Self::Bus => !tram,
            Self::Tram => tram,
        }
    }

    /// Whether two scopes can be in use on the same vehicle, so their keys must differ.
    #[must_use]
    pub fn overlaps(self, other: Self) -> bool {
        self == Self::Both || other == Self::Both || self == other
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
