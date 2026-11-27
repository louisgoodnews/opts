//! The stitched border normals of the terrain meshes (`docs/src/architecture/terrain-rendering.md`
//! section 2, Q-TR-01).
//!
//! A tile's normals take the heights one cell beyond each edge from the neighbours that are built.
//! The second tile of a pair rebuilds the shared border: when a tile is built, each built
//! neighbour's normals are recomputed with its heights, so both sides of a border light it from
//! the same slope whichever of the two was built first. A neighbour that is not built yet is a
//! one-sided difference until it is.

use std::sync::Arc;

use bevy::prelude::*;
use opts_bevy::terrain::{apply_field_normals, TileBeyond};
use opts_core::formats::map::MapTerrain;

use super::super::WorldLoadJob;

/// A built tile's terrain mesh and the grid it was built from: what a neighbour needs to stitch
/// the border it shares.
pub(in super::super) struct TerrainRecord {
    pub(super) mesh: Handle<Mesh>,
    pub(super) grid: Arc<MapTerrain>,
    pub(super) cell: f32,
}

/// West, east, south and north neighbour of a tile (north is `+y`, as in the tile origins).
const EDGES: [(i32, i32); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];

/// The beyond-heights of tile `key` (a grid of `cells` cells per side) from its built neighbours.
pub(in super::super) fn beyond_of(job: &WorldLoadJob, key: (i32, i32), cells: usize) -> TileBeyond {
    let held = EDGES.map(|(dx, dy)| {
        job.terrain_records
            .get(&(key.0 + dx, key.1 + dy))
            .map(|r| r.grid.as_ref())
    });
    TileBeyond::from_neighbours(cells, held[0], held[1], held[2], held[3])
}

/// Records the terrain mesh of a tile that has just been built, and restitches the borders it
/// shares with its built neighbours: their normals are recomputed with this tile's heights.
pub(in super::super) fn stitch_built_tile(
    job: &mut WorldLoadJob,
    meshes: &mut Assets<Mesh>,
    key: (i32, i32),
    record: TerrainRecord,
) {
    job.terrain_records.insert(key, record);
    for (dx, dy) in EDGES {
        let other = (key.0 + dx, key.1 + dy);
        let Some(rec) = job.terrain_records.get(&other) else {
            continue;
        };
        let beyond = beyond_of(job, other, rec.grid.grid_cells as usize);
        if let Some(mesh) = meshes.get_mut(&rec.mesh) {
            apply_field_normals(mesh, &rec.grid, rec.cell, &beyond);
        }
    }
}
