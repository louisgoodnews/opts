//! Spatial zones per tile (M-0115): the zone registry follows the resident tiles, so a tile that
//! streams in gets its zone entities and a tile that streams out, is reloaded or leaves with the
//! world loses them.

use bevy::prelude::*;
use opts_bevy::spatial_zones::ceiling::{covered_zones, SplineCandidate};
use opts_bevy::spatial_zones::{SpatialZone, ZoneRegistry};

use crate::in_game::omsi_world::WorldLoadJob;

/// The zones of a tile, derived from the placed splines of the tile by the covered-section rule
/// (M-0115). The tile loader does not yet keep the parsed splines, so the game passes none and
/// every tile has no zones until that feed is wired.
#[must_use]
pub fn detect_tile_zones(_tile: (i32, i32), splines: &[SplineCandidate]) -> Vec<SpatialZone> {
    covered_zones(splines)
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
    registry.reconcile(&mut commands, &resident, |tile| {
        detect_tile_zones(tile, &[])
    });
}
