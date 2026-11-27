//! Terrain normals from the height field, stitched across tile borders (M-0025 Phase B;
//! `docs/src/architecture/terrain-rendering.md` section 2).
//!
//! A vertex's normal is the central difference of its height along the two grid axes, the same
//! surface the height queries use. At a tile edge the neighbour's heights one cell beyond the
//! border stand in for the missing side, so the two tiles sharing a border light it from the same
//! slope. A side that is not held falls back to a one-sided difference.

use super::*;
use bevy::mesh::VertexAttributeValues;

/// The heights one cell beyond each edge of a tile, for each neighbour that is held: the
/// neighbour's grid column (or row) next to the shared border, one value per grid point along it.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TileBeyond {
    /// The column next to the west edge, by row `j`.
    pub west: Option<Vec<f32>>,
    /// The column next to the east edge, by row `j`.
    pub east: Option<Vec<f32>>,
    /// The row next to the south edge, by column `i`.
    pub south: Option<Vec<f32>>,
    /// The row next to the north edge, by column `i`.
    pub north: Option<Vec<f32>>,
}

impl TileBeyond {
    /// The beyond-heights of four neighbours of a tile with `cells` cells per side. A neighbour
    /// with another grid size, or a malformed one, counts as not held.
    #[must_use]
    pub fn from_neighbours(
        cells: usize,
        west: Option<&MapTerrain>,
        east: Option<&MapTerrain>,
        south: Option<&MapTerrain>,
        north: Option<&MapTerrain>,
    ) -> Self {
        if cells == 0 {
            return Self::default();
        }
        let dim = cells + 1;
        let usable = |t: Option<&MapTerrain>| {
            t.filter(|t| t.grid_cells as usize == cells && t.heights.len() >= dim * dim)
        };
        let at = |t: &MapTerrain, i: usize, j: usize| t.heights[j * dim + i];
        Self {
            west: usable(west).map(|t| (0..dim).map(|j| at(t, cells - 1, j)).collect()),
            east: usable(east).map(|t| (0..dim).map(|j| at(t, 1, j)).collect()),
            south: usable(south).map(|t| (0..dim).map(|i| at(t, i, cells - 1)).collect()),
            north: usable(north).map(|t| (0..dim).map(|i| at(t, i, 1)).collect()),
        }
    }
}

/// Unit normals of a tile's grid points in Bevy axes (`x` east, `y` up, `z` south), in the order
/// of the mesh's vertices. `cell` is the edge of one cell (m).
#[must_use]
pub fn field_normals(terrain: &MapTerrain, cell: f32, beyond: &TileBeyond) -> Vec<[f32; 3]> {
    let n = terrain.grid_cells as usize;
    let dim = n + 1;
    let h = |i: usize, j: usize| terrain.heights.get(j * dim + i).copied().unwrap_or(0.0);
    let mut out = Vec::with_capacity(dim * dim);
    for j in 0..dim {
        for i in 0..dim {
            let west = if i > 0 {
                Some(h(i - 1, j))
            } else {
                beyond.west.as_ref().and_then(|c| c.get(j).copied())
            };
            let east = if i < n {
                Some(h(i + 1, j))
            } else {
                beyond.east.as_ref().and_then(|c| c.get(j).copied())
            };
            let south = if j > 0 {
                Some(h(i, j - 1))
            } else {
                beyond.south.as_ref().and_then(|r| r.get(i).copied())
            };
            let north = if j < n {
                Some(h(i, j + 1))
            } else {
                beyond.north.as_ref().and_then(|r| r.get(i).copied())
            };
            let here = h(i, j);
            let d_east = slope(west, east, here, cell);
            // Bevy's z runs south, so the north slope enters with the opposite sign.
            let d_north = slope(south, north, here, cell);
            out.push(Vec3::new(-d_east, 1.0, d_north).normalize().to_array());
        }
    }
    out
}

/// Writes the field normals of a tile's grid over the first normals of `mesh`. The grid's vertices
/// come first in the terrain meshes (plain and cut), so the vertices after them keep theirs.
pub fn apply_field_normals(mesh: &mut Mesh, terrain: &MapTerrain, cell: f32, beyond: &TileBeyond) {
    let normals = field_normals(terrain, cell, beyond);
    if let Ok(Some(VertexAttributeValues::Float32x3(values))) =
        mesh.try_attribute_mut_option(Mesh::ATTRIBUTE_NORMAL)
    {
        for (value, normal) in values.iter_mut().zip(normals) {
            *value = normal;
        }
    }
}

/// Slope along one axis from the heights before and after a point.
fn slope(before: Option<f32>, after: Option<f32>, here: f32, cell: f32) -> f32 {
    match (before, after) {
        (Some(p), Some(q)) => (q - p) / (2.0 * cell),
        (Some(p), None) => (here - p) / cell,
        (None, Some(q)) => (q - here) / cell,
        (None, None) => 0.0,
    }
}
