//! The tile terrain component and the terrain mesh built from the map's heights.

use super::*;

/// A tile's terrain grid with the size of its cells (`TileGrid::cell` of the tile's row): what
/// every height consumer samples.
#[derive(Debug, Clone, PartialEq)]
pub struct TileTerrain {
    /// The height grid.
    pub grid: MapTerrain,
    /// Edge of one cell (m).
    pub cell: f32,
    /// The strips of aligned splines the terrain is shaped to ([`cut`]), when there are any.
    pub cut: Option<std::sync::Arc<TerrainCut>>,
}

impl TileTerrain {
    /// A grid with cells of `cell` metres.
    #[must_use]
    pub fn new(grid: MapTerrain, cell: f32) -> Self {
        Self {
            grid,
            cell,
            cut: None,
        }
    }

    /// The same terrain shaped to the aligned strips of `cut` (none when it is empty).
    #[must_use]
    pub fn with_cut(mut self, cut: TerrainCut) -> Self {
        self.cut = (!cut.is_empty()).then(|| std::sync::Arc::new(cut));
        self
    }

    /// Ground height at tile-local `(x, y)` metres, as drawn: an aligned strip's where one lies,
    /// else the grid's ([`terrain_height`]).
    #[must_use]
    pub fn height(&self, x: f32, y: f32) -> f32 {
        self.cut
            .as_ref()
            .and_then(|c| c.height(x, y))
            .unwrap_or_else(|| terrain_height(&self.grid, self.cell, x, y))
    }
}

/// Height of a tile's terrain at tile-local `(x, y)` metres, interpolated on the triangles the
/// mesh is drawn with ([`build_terrain_mesh`]: cell `(i, j)` splits into `(i,j) (i+1,j) (i,j+1)`
/// and `(i+1,j) (i+1,j+1) (i,j+1)`), so objects, lanes and vehicles stand on the drawn ground.
/// Points outside the grid take the height at the nearest border.
#[must_use]
pub fn terrain_height(terrain: &MapTerrain, cell: f32, x: f32, y: f32) -> f32 {
    let n = terrain.grid_cells as usize;
    let dim = n + 1;
    if n == 0 || terrain.heights.len() < dim * dim {
        return 0.0;
    }
    let gx = (x / cell).clamp(0.0, n as f32);
    let gy = (y / cell).clamp(0.0, n as f32);
    let (i, j) = ((gx as usize).min(n - 1), (gy as usize).min(n - 1));
    let (u, w) = (gx - i as f32, gy - j as f32);
    let h = |i: usize, j: usize| terrain.heights[j * dim + i];
    if u + w <= 1.0 {
        let a = h(i, j);
        a + (h(i + 1, j) - a) * u + (h(i, j + 1) - a) * w
    } else {
        let d = h(i + 1, j + 1);
        d + (h(i, j + 1) - d) * (1.0 - u) + (h(i + 1, j) - d) * (1.0 - w)
    }
}

/// Builds the terrain mesh of one tile in absolute Bevy world coordinates, without neighbours.
///
/// `origin` is the tile's south-west corner in OMSI world metres `(east, north)`, `cell` its
/// terrain cell. With `uv_per_tile` the UVs span 0..1 over the tile (painted ground texture),
/// otherwise one unit per cell (a repeating base texture).
#[must_use]
pub fn build_terrain_mesh(
    terrain: &MapTerrain,
    origin: (f32, f32),
    cell: f32,
    uv_per_tile: bool,
) -> Mesh {
    build_terrain_mesh_with(terrain, origin, cell, uv_per_tile, &TileBeyond::default())
}

/// [`build_terrain_mesh`] with the normals stitched to the held neighbours (see [`field_normals`]).
#[must_use]
pub fn build_terrain_mesh_with(
    terrain: &MapTerrain,
    origin: (f32, f32),
    cell: f32,
    uv_per_tile: bool,
    beyond: &TileBeyond,
) -> Mesh {
    let dim = terrain.grid_cells as usize + 1;
    let mut positions = Vec::with_capacity(dim * dim);
    let mut uvs = Vec::with_capacity(dim * dim);
    for j in 0..dim {
        for i in 0..dim {
            let h = terrain.heights.get(j * dim + i).copied().unwrap_or(0.0);
            positions.push([origin.0 + i as f32 * cell, h, -(origin.1 + j as f32 * cell)]);
            if uv_per_tile {
                // One texture over the whole tile, row 0 at the north edge.
                let n = (dim - 1) as f32;
                uvs.push([i as f32 / n, 1.0 - j as f32 / n]);
            } else {
                uvs.push([i as f32, j as f32]);
            }
        }
    }
    let mut indices = Vec::with_capacity((dim - 1) * (dim - 1) * 6);
    for j in 0..dim - 1 {
        for i in 0..dim - 1 {
            let a = (j * dim + i) as u32;
            let b = a + 1;
            let c = a + dim as u32;
            let d = c + 1;
            indices.extend_from_slice(&[a, b, c, b, d, c]);
        }
    }
    let mut mesh = smooth_normal_mesh(positions, uvs, indices);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, field_normals(terrain, cell, beyond));
    mesh
}
