//! Ground following of the player's vehicle: road surface and terrain height under it.
//!
//! The simulation is planar; this module gives the chassis an elevation and a pitch from the
//! loaded content, so the bus follows slopes instead of keeping its spawn height (and diving into
//! the terrain on the first hill).

use bevy::prelude::*;
use opts_bevy::scenery::spline_pose;
use opts_bevy::TileTerrain;
use opts_core::formats::map::{MapSplineInstance, TileGrid};
use opts_core::formats::scenery::SplineProfile;
use opts_sim::collision::{ColliderGrid, Obb};
use std::collections::HashMap;

/// Spacing of the road centre-line samples (m).
const SAMPLE_STEP_M: f32 = 2.0;
/// Cell size of the sample index (m).
const CELL_M: f32 = 8.0;
/// How much closer than a road layer the terrain must be to the vehicle's height to win (m).
const TERRAIN_LAYER_MARGIN_M: f32 = 0.3;
/// A road layer this far above the vehicle is a deck over it, not the ground (m).
const DECK_GAP_M: f32 = 1.0;

mod index;
mod insert;
mod query;
mod sample;

use index::cell_of;
pub use index::GroundIndex;
use sample::RoadSample;

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

#[cfg(test)]
mod tests;
