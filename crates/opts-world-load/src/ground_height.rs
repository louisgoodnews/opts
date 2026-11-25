//! Ground following of the player's vehicle: road surface and terrain height under it.
//!
//! The simulation is planar; this module gives the chassis an elevation and a pitch from the
//! loaded content, so the bus follows slopes instead of keeping its spawn height (and diving into
//! the terrain on the first hill). The index itself is a state type in `opts-game-state`; this
//! module keeps the systems that fill and drop it.

use bevy::prelude::*;
use opts_bevy::TileTerrain;
use opts_core::formats::map::{MapSplineInstance, TileGrid};
use opts_core::formats::scenery::SplineProfile;

pub use opts_game_state::in_game::omsi_world::ground_height::GroundIndex;

/// Registers a built tile with the ground index.
pub(super) fn register(
    commands: &mut Commands,
    key: (i32, i32),
    origin: (f32, f32),
    terrain: Option<TileTerrain>,
    roads: Vec<(MapSplineInstance, SplineProfile)>,
    start_tile: (i32, i32),
    grid: TileGrid,
) {
    commands.queue(move |world: &mut World| {
        if let Some(mut g) = world.get_resource_mut::<GroundIndex>() {
            g.start_tile = start_tile;
            g.grid = grid;
            g.insert_tile(key, origin, terrain, &roads);
        }
    });
}

/// Removes an unloaded tile.
pub(super) fn unregister(commands: &mut Commands, key: (i32, i32)) {
    commands.queue(move |world: &mut World| {
        if let Some(mut g) = world.get_resource_mut::<GroundIndex>() {
            g.remove_tile(key);
        }
    });
}
