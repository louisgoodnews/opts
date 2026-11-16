//! Spatial zones per tile (M-0115): the zone registry follows the resident tiles, so a tile that
//! streams in gets its zone entities and a tile that streams out, is reloaded or leaves with the
//! world loses them.

use bevy::prelude::*;
use opts_bevy::spatial_zones::{SpatialZone, ZoneRegistry};

use crate::in_game::omsi_world::WorldLoadJob;

/// The zones of a tile. The detector that derives them from the tile data is still open (M-0115),
/// so every tile has none yet and only the bookkeeping runs.
#[must_use]
pub fn detect_tile_zones(_tile: (i32, i32)) -> Vec<SpatialZone> {
    Vec::new()
}

/// Brings the zone registry in line with the resident tiles of the world. Without a world every
/// registered tile is dropped.
pub fn sync_tile_zones(
    job: Option<Res<WorldLoadJob>>,
    mut registry: ResMut<ZoneRegistry>,
    mut commands: Commands,
) {
    let resident: Vec<(i32, i32)> =
        job.map_or_else(Vec::new, |job| job.resident.keys().copied().collect());
    registry.reconcile(&mut commands, &resident, detect_tile_zones);
}
