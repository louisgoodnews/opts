//! Terrain height grids (`.map.terrain`) → Bevy meshes and height sampling.

use bevy::asset::Handle;
use bevy::pbr::StandardMaterial;
use bevy::render::mesh::Mesh;
use bevy::render::render_resource::Face;
use bevy::render::texture::Image;

use opts_core::formats::map::MapTerrain;

use crate::mesh::smooth_normal_mesh;

/// Side length of a map tile in metres (confirmed, `docs/src/formats/map/tiles.md` Q-M01).
pub const TILE_SIZE_M: f32 = 300.0;

/// Terrain grid cell size in metres (inferred from 60 cells per 300 m tile).
pub const TERRAIN_CELL_M: f32 = TILE_SIZE_M / 60.0;

/// Bilinearly samples a tile's height grid at tile-local `(x, y)` metres.
#[must_use]
pub fn sample_terrain(terrain: &MapTerrain, x: f32, y: f32) -> f32 {
    let dim = terrain.grid_cells as usize + 1;
    if dim < 2 || terrain.heights.len() < dim * dim {
        return 0.0;
    }
    let max = terrain.grid_cells as f32 - 0.0001;
    let gx = (x / TERRAIN_CELL_M).clamp(0.0, max);
    let gy = (y / TERRAIN_CELL_M).clamp(0.0, max);
    let (ix, iy) = (gx as usize, gy as usize);
    let (fx, fy) = (gx - ix as f32, gy - iy as f32);
    let h = |i: usize, j: usize| terrain.heights[j * dim + i];
    let a = h(ix, iy) * (1.0 - fx) + h(ix + 1, iy) * fx;
    let b = h(ix, iy + 1) * (1.0 - fx) + h(ix + 1, iy + 1) * fx;
    a * (1.0 - fy) + b * fy
}

/// Builds the terrain mesh of one tile in absolute Bevy world coordinates.
///
/// `origin` is the tile's south-west corner in OMSI world metres `(east, north)`. With
/// `uv_per_tile` the UVs span 0..1 over the tile (painted ground texture), otherwise one unit per
/// 5 m cell (a repeating base texture).
#[must_use]
pub fn build_terrain_mesh(terrain: &MapTerrain, origin: (f32, f32), uv_per_tile: bool) -> Mesh {
    let dim = terrain.grid_cells as usize + 1;
    let mut positions = Vec::with_capacity(dim * dim);
    let mut uvs = Vec::with_capacity(dim * dim);
    for j in 0..dim {
        for i in 0..dim {
            let h = terrain.heights.get(j * dim + i).copied().unwrap_or(0.0);
            positions.push([
                origin.0 + i as f32 * TERRAIN_CELL_M,
                h,
                -(origin.1 + j as f32 * TERRAIN_CELL_M),
            ]);
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
    smooth_normal_mesh(positions, uvs, indices)
}

#[must_use]
pub fn terrain_material(texture: Option<Handle<Image>>) -> StandardMaterial {
    StandardMaterial {
        base_color_texture: texture,
        perceptual_roughness: 0.95,
        metallic: 0.0,
        cull_mode: Some(Face::Back),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terrain_sampling_interpolates() {
        let dim = 61;
        let mut heights = vec![0.0; dim * dim];
        for j in 0..dim {
            for i in 0..dim {
                heights[j * dim + i] = i as f32;
            }
        }
        let t = MapTerrain {
            grid_cells: 60,
            heights,
        };
        assert!((sample_terrain(&t, 7.5, 100.0) - 1.5).abs() < 1e-4);
        let mesh = build_terrain_mesh(&t, (300.0, 0.0), false);
        assert_eq!(mesh.count_vertices(), 61 * 61);
    }
}
