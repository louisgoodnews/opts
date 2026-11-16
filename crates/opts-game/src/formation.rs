//! Train formations in the game: resolving a [`TrainFormation`] against the installation, and the
//! control-protocol methods `formation.validate` and `formation.spawn`.
//! Design: `docs/src/architecture/train-formation-editor.md`, ADR-0013.

pub mod store;

mod library;
mod lookup;
mod methods;
mod request;
mod resolve;

use bevy::prelude::*;
use opts_bevy::control::state::ControlRegistry;

use store::FormationStore;

pub use lookup::JobLookup;
pub use resolve::{queue_spawn, resolve, ResolvedFormation};

/// Registers the methods.
pub fn register(registry: &mut ControlRegistry) {
    registry.register("formation.validate", methods::validate);
    registry.register("formation.spawn", methods::spawn);
    registry.register("formation.save", library::save);
    registry.register("formation.list", library::list);
    registry.register("formation.delete", library::delete);
}

/// Where the user's formations are kept. Replaced in tests; the default is the user data folder.
#[derive(Resource, Debug, Clone)]
pub struct FormationLibrary(pub Option<FormationStore>);

impl Default for FormationLibrary {
    fn default() -> Self {
        Self(FormationStore::user())
    }
}

#[cfg(test)]
mod tests;
