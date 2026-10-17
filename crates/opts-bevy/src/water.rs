//! Water surface of a map tile (`.map.water`): one patch whose heights follow the four corner
//! levels. Specification: `docs/src/architecture/terrain-rendering.md`, sections 6 and 7.

use bevy::mesh::{Indices, Mesh, PrimitiveTopology};
use bevy::asset::RenderAssetUsages;
use bevy::color::Color;
use bevy::pbr::StandardMaterial;
use bevy::prelude::AlphaMode;
use opts_core::formats::map::{MapTerrain, MapWater};

/// Quads per tile edge of the water patch (the bilinear surface is sampled on this grid).
pub const WATER_GRID: usize = 8;

/// Water level at the tile-relative position `(u, v)`, both in `0..=1` (`u` east, `v` north): the
/// bilinear interpolation of the corner levels `[south-west, south-east, north-west, north-east]`.
#[must_use]
pub fn water_level_at(levels: [f32; 4], u: f32, v: f32) -> f32 {
    (1.0 - u) * (1.0 - v) * levels[0]
        + u * (1.0 - v) * levels[1]
        + (1.0 - u) * v * levels[2]
        + u * v * levels[3]
}

/// Whether the water of a tile can be seen at all: the water surface lies above the terrain at
/// some grid point. A tile where it does not gets no water mesh.
#[must_use]
pub fn water_visible(water: &MapWater, terrain: &MapTerrain) -> bool {
    let cells = terrain.grid_cells as usize;
    let dim = cells + 1;
    if cells == 0 || terrain.heights.len() < dim * dim {
        return false;
    }
    (0..dim * dim).any(|k| {
        let (u, v) = ((k % dim) as f32 / cells as f32, (k / dim) as f32 / cells as f32);
        terrain.heights[k] < water_level_at(water.levels, u, v)
    })
}

/// Builds the water patch of one tile in the session frame.
///
/// `origin` is the tile's south-west corner in metres `(east, north)` relative to the session
/// origin and `size` the tile's side length; the vertex layout follows the terrain mesh (Bevy axes,
/// `z = -north`, counter-clockwise seen from above), UVs span 0..1 over the tile.
#[must_use]
pub fn build_water_mesh(levels: [f32; 4], origin: (f32, f32), size: f32) -> Mesh {
    let dim = WATER_GRID + 1;
    let mut positions = Vec::with_capacity(dim * dim);
    let mut uvs = Vec::with_capacity(dim * dim);
    for j in 0..dim {
        for i in 0..dim {
            let (u, v) = (i as f32 / WATER_GRID as f32, j as f32 / WATER_GRID as f32);
            positions.push([
                origin.0 + u * size,
                water_level_at(levels, u, v),
                -(origin.1 + v * size),
            ]);
            uvs.push([u, 1.0 - v]);
        }
    }
    let mut indices = Vec::with_capacity(WATER_GRID * WATER_GRID * 6);
    for j in 0..WATER_GRID {
        for i in 0..WATER_GRID {
            let a = (j * dim + i) as u32;
            let (b, c) = (a + 1, a + dim as u32);
            let d = c + 1;
            indices.extend_from_slice(&[a, b, c, b, d, c]);
        }
    }
    // The surface is smooth and nearly level: one normal from the corner levels.
    let slope_x = ((levels[1] + levels[3]) - (levels[0] + levels[2])) * 0.5 / size;
    let slope_z = ((levels[2] + levels[3]) - (levels[0] + levels[1])) * 0.5 / size;
    let normal = bevy::math::Vec3::new(-slope_x, 1.0, slope_z).normalize().to_array();
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    let normals = vec![normal; positions.len()];
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

/// First-version water material: translucent blue-green, smooth, seen from both sides. The
/// physically calibrated look belongs to Phase 8 (M-0060).
#[must_use]
pub fn water_material() -> StandardMaterial {
    StandardMaterial {
        base_color: Color::srgba(0.09, 0.28, 0.34, 0.62),
        alpha_mode: AlphaMode::Blend,
        perceptual_roughness: 0.06,
        metallic: 0.0,
        reflectance: 0.55,
        cull_mode: None,
        double_sided: true,
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flat_terrain(h: f32) -> MapTerrain {
        MapTerrain {
            grid_cells: 60,
            heights: vec![h; 61 * 61],
        }
    }

    #[test]
    fn corner_levels_are_interpolated_bilinearly() {
        let l = [0.0, 10.0, 20.0, 30.0]; // SW, SE, NW, NE
        assert_eq!(water_level_at(l, 0.0, 0.0), 0.0);
        assert_eq!(water_level_at(l, 1.0, 0.0), 10.0);
        assert_eq!(water_level_at(l, 0.0, 1.0), 20.0);
        assert_eq!(water_level_at(l, 1.0, 1.0), 30.0);
        assert!((water_level_at(l, 0.5, 0.5) - 15.0).abs() < 1e-5);
        assert!((water_level_at(l, 0.25, 0.0) - 2.5).abs() < 1e-5);
    }

    #[test]
    fn water_is_visible_only_where_it_lies_above_the_terrain() {
        let water = MapWater {
            version: 1,
            levels: [-5.0; 4],
        };
        assert!(!water_visible(&water, &flat_terrain(0.0)), "default level under flat land");
        assert!(water_visible(&water, &flat_terrain(-10.0)), "dug terrain holds water");
        // one corner of the terrain dips below the level
        let mut t = flat_terrain(0.0);
        t.heights[0] = -6.0;
        assert!(water_visible(&water, &t));
    }

    #[test]
    fn mesh_spans_the_tile_with_matching_edge_heights() {
        let levels = [1.0, 2.0, 3.0, 4.0];
        let mesh = build_water_mesh(levels, (300.0, 600.0), 300.0);
        assert_eq!(mesh.count_vertices(), (WATER_GRID + 1) * (WATER_GRID + 1));
        let pos = mesh
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .and_then(|a| a.as_float3())
            .unwrap();
        // south-west corner first, north-east corner last (Bevy z = -north)
        assert_eq!(pos[0], [300.0, 1.0, -600.0]);
        assert_eq!(*pos.last().unwrap(), [600.0, 4.0, -900.0]);
        // the east edge of this tile and the west edge of its east neighbour share heights when
        // the neighbour's west corners are this tile's east corners
        let east = build_water_mesh([2.0, 5.0, 4.0, 6.0], (600.0, 600.0), 300.0);
        let east_pos = east
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .and_then(|a| a.as_float3())
            .unwrap();
        let dim = WATER_GRID + 1;
        for j in 0..dim {
            let (a, b) = (pos[j * dim + WATER_GRID], east_pos[j * dim]);
            assert_eq!(a[0], b[0]);
            assert!((a[1] - b[1]).abs() < 1e-5, "row {j}: {a:?} vs {b:?}");
        }
    }

    #[test]
    fn triangles_face_up() {
        let mesh = build_water_mesh([0.0; 4], (0.0, 0.0), 100.0);
        let pos = mesh.attribute(Mesh::ATTRIBUTE_POSITION).and_then(|a| a.as_float3()).unwrap();
        let Some(Indices::U32(idx)) = mesh.indices() else { panic!("indices") };
        for t in idx.chunks_exact(3) {
            let p = |k: u32| bevy::math::Vec3::from(pos[k as usize]);
            let n = (p(t[1]) - p(t[0])).cross(p(t[2]) - p(t[0]));
            assert!(n.y > 0.0, "{t:?} faces down");
        }
    }
}
