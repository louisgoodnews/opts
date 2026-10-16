//! Water surfaces (`.map.water`) → Bevy meshes.
//!
//! A tile's water is a surface whose height is the bilinear blend of four corner levels
//! (`docs/src/formats/map/tiles.md`, `.map.water`). It is drawn only over the terrain cells it
//! covers: a cell gets two triangles when the water is above the terrain at any of its corners,
//! so the surface ends under the shore and needs no clipping against the ground.

use bevy::color::Color;
use bevy::mesh::Mesh;
use bevy::pbr::StandardMaterial;
use bevy::prelude::AlphaMode;

use opts_core::formats::map::{MapTerrain, MapWater};

use crate::mesh::smooth_normal_mesh;
use crate::terrain::terrain_cell;

/// Water surface of one tile in absolute Bevy world coordinates, or `None` when the water is
/// nowhere above the terrain.
///
/// `origin` is the tile's south-west corner in OMSI world metres `(east, north)`, as for
/// [`crate::build_terrain_mesh`].
#[must_use]
pub fn build_water_mesh(
    terrain: &MapTerrain,
    water: &MapWater,
    origin: (f32, f32),
) -> Option<Mesh> {
    let cells = terrain.grid_cells as usize;
    let dim = cells + 1;
    if cells == 0 || terrain.heights.len() < dim * dim {
        return None;
    }
    let ground = |i: usize, j: usize| terrain.heights[j * dim + i];
    let level =
        |i: usize, j: usize| water.level_at(i as f32 / cells as f32, j as f32 / cells as f32);
    let wet = |i: usize, j: usize| level(i, j) > ground(i, j);

    // Vertices are created on demand for the corners of wet cells.
    let mut index_of = vec![u32::MAX; dim * dim];
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut uvs: Vec<[f32; 2]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    let mut vertex =
        |i: usize, j: usize, positions: &mut Vec<[f32; 3]>, uvs: &mut Vec<[f32; 2]>| {
            let slot = &mut index_of[j * dim + i];
            if *slot == u32::MAX {
                *slot = positions.len() as u32;
                positions.push([
                    origin.0 + i as f32 * terrain_cell(),
                    level(i, j),
                    -(origin.1 + j as f32 * terrain_cell()),
                ]);
                uvs.push([i as f32, j as f32]);
            }
            *slot
        };
    for j in 0..cells {
        for i in 0..cells {
            if !(wet(i, j) || wet(i + 1, j) || wet(i, j + 1) || wet(i + 1, j + 1)) {
                continue;
            }
            let a = vertex(i, j, &mut positions, &mut uvs);
            let b = vertex(i + 1, j, &mut positions, &mut uvs);
            let c = vertex(i, j + 1, &mut positions, &mut uvs);
            let d = vertex(i + 1, j + 1, &mut positions, &mut uvs);
            // Same winding as the terrain: seen from above the triangles face up.
            indices.extend_from_slice(&[a, b, c, b, d, c]);
        }
    }
    (!indices.is_empty()).then(|| smooth_normal_mesh(positions, uvs, indices))
}

/// A translucent blue-green surface: the colour of the water seen at a slant, with a little
/// sheen. Reflection and foam are not part of it.
#[must_use]
pub fn water_material() -> StandardMaterial {
    StandardMaterial {
        base_color: Color::srgba(0.07, 0.24, 0.30, 0.72),
        alpha_mode: AlphaMode::Blend,
        perceptual_roughness: 0.12,
        metallic: 0.0,
        reflectance: 0.6,
        cull_mode: None,
        double_sided: true,
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flat_terrain(height: f32) -> MapTerrain {
        MapTerrain {
            grid_cells: 60,
            heights: vec![height; 61 * 61],
        }
    }

    #[test]
    fn dry_land_gets_no_water() {
        let w = MapWater {
            version: 1,
            levels: [-5.0; 4],
        };
        assert!(build_water_mesh(&flat_terrain(10.0), &w, (0.0, 0.0)).is_none());
    }

    #[test]
    fn a_flooded_tile_is_covered_completely() {
        let w = MapWater {
            version: 1,
            levels: [3.0; 4],
        };
        let mesh = build_water_mesh(&flat_terrain(0.0), &w, (0.0, 0.0)).expect("flooded");
        assert_eq!(mesh.count_vertices(), 61 * 61);
        assert_eq!(mesh.indices().expect("indexed").len(), 60 * 60 * 6);
    }

    #[test]
    fn water_stops_where_the_land_rises() {
        // Terrain rises from 0 m in the west to 12 m in the east: a level of 6 m covers the
        // western half only.
        let mut t = flat_terrain(0.0);
        for j in 0..61 {
            for i in 0..61 {
                t.heights[j * 61 + i] = i as f32 * 0.2;
            }
        }
        let w = MapWater {
            version: 1,
            levels: [6.0; 4],
        };
        let mesh = build_water_mesh(&t, &w, (0.0, 0.0)).expect("partly wet");
        let n = mesh.indices().expect("indexed").len() / 6;
        // Columns 0..30 are wet (the ground at column 30 is exactly 6 m, so column 29's cell
        // reaches it): 30 columns of 60 cells.
        assert_eq!(n, 30 * 60);
    }

    #[test]
    fn a_sloping_surface_follows_the_corner_levels() {
        let w = MapWater {
            version: 1,
            levels: [0.0, 6.0, 0.0, 6.0],
        };
        let mesh = build_water_mesh(&flat_terrain(-1.0), &w, (0.0, 0.0)).expect("wet");
        let pos = mesh
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .and_then(|a| a.as_float3())
            .expect("positions");
        let max = pos.iter().map(|p| p[1]).fold(f32::MIN, f32::max);
        let min = pos.iter().map(|p| p[1]).fold(f32::MAX, f32::min);
        assert!((max - 6.0).abs() < 1e-4 && min.abs() < 1e-4, "{min} {max}");
    }
}
