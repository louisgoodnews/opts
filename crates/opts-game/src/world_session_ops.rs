//! The session-vehicle side of the world load (ISS-064): `opts-world-load` calls the session
//! vehicle operations through its `SessionVehicleOps` trait, and this forwards each call to the
//! function it stands for, with the same arguments.

use bevy::prelude::*;
use opts_game_state::session_vehicles::VehicleId;
use opts_world_load::session_ops::SessionVehicleOps;

use crate::session_vehicles::ops;

/// The session-vehicle operations of the game: the `session_vehicles::ops` functions.
#[derive(Debug, Clone, Copy, Default)]
pub struct GameSessionVehicleOps;

impl SessionVehicleOps for GameSessionVehicleOps {
    fn switch_to(&self, world: &mut World, id: VehicleId) -> Result<(), String> {
        ops::switch_to(world, id).map_err(|e| e.to_string())
    }
}
