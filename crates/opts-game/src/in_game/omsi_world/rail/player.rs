//! The player's rail vehicle: the [`RailPlayer`] component (its state type lives in
//! `opts-game-state`) and the rule that tells a rail vehicle from a road one.

use opts_core::formats::vehicles::BusManifest;

pub use opts_game_state::in_game::omsi_world::rail::RailPlayer;

/// Whether a manifest is a rail vehicle.
#[must_use]
pub fn is_rail(manifest: &BusManifest) -> bool {
    manifest.vehicle_type == Some(2)
}
