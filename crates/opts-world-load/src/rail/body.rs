//! The query of the player's rail step.

use super::*;

/// What the rail step reads and writes of the player's vehicle.
pub(super) type RailBody = (
    &'static mut VehicleController,
    &'static mut RailPlayer,
    Option<&'static opts_bevy::RailControls>,
    Option<&'static opts_bevy::ActiveCab>,
    Option<&'static opts_bevy::ScriptConsist>,
);
