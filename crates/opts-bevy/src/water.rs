//! Water surfaces (`.map.water`) → Bevy meshes.
//!
//! A tile's water is a surface whose height is the bilinear blend of four corner levels
//! (`docs/src/formats/map/tiles.md`, `.map.water`). It is drawn only over the terrain cells it
//! covers: a cell gets two triangles when the water is above the terrain at any of its corners,
//! so the surface ends under the shore and needs no clipping against the ground.
//!
//! The surface is tinted by depth (M-0067): every vertex carries a colour whose alpha fades from
//! nothing at the shore to [`DEEP_ALPHA`] and whose rgb darkens from the shallow tint of
//! [`water_material`] to the deep one, over [`FULL_DEPTH_M`] of water above the ground. The fade
//! is what a shore looks like where the surface cuts the terrain, without clipping.

use bevy::color::Color;
use bevy::mesh::Mesh;
use bevy::pbr::StandardMaterial;
use bevy::prelude::AlphaMode;

/// Depth (m) of water above the ground at which the surface has its deep colour.
pub const FULL_DEPTH_M: f32 = 6.0;
/// Depth (m) over which the surface fades in from the shore.
const SHORE_FADE_M: f32 = 0.5;
/// Opacity at the shore end of the tint (just past the fade) and in deep water.
const SHALLOW_ALPHA: f32 = 0.4;
/// Opacity of deep water.
pub const DEEP_ALPHA: f32 = 0.92;
/// The shallow tint (the material's base colour) and the deep one the vertex colours darken to.
const SHALLOW_RGB: [f32; 3] = [0.12, 0.38, 0.42];
const DEEP_RGB: [f32; 3] = [0.05, 0.22, 0.32];

/// The vertex colour for water `depth` metres above the ground: multiplies the material's
/// base colour, so rgb is the ratio of the deep tint to the shallow one.
#[must_use]
pub fn depth_tint(depth: f32) -> [f32; 4] {
    let shore = (depth / SHORE_FADE_M).clamp(0.0, 1.0);
    let t = (depth / FULL_DEPTH_M).clamp(0.0, 1.0);
    let t = t * t * (3.0 - 2.0 * t);
    let rgb = |k: usize| 1.0 + (DEEP_RGB[k] / SHALLOW_RGB[k] - 1.0) * t;
    let alpha = shore * (SHALLOW_ALPHA + (DEEP_ALPHA - SHALLOW_ALPHA) * t);
    [rgb(0), rgb(1), rgb(2), alpha]
}

use opts_core::formats::map::{MapTerrain, MapWater};

use crate::mesh::smooth_normal_mesh;

/// Water surface of one tile in absolute Bevy world coordinates, or `None` when the water is
/// nowhere above the terrain.
///
/// `origin` is the tile's south-west corner in OMSI world metres `(east, north)` and `cell` its
/// terrain cell, as for [`crate::build_terrain_mesh`].
#[must_use]
pub fn build_water_mesh(
    terrain: &MapTerrain,
    water: &MapWater,
    origin: (f32, f32),
    cell: f32,
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
    let mut colors: Vec<[f32; 4]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    let mut vertex =
        |i: usize, j: usize, positions: &mut Vec<[f32; 3]>, uvs: &mut Vec<[f32; 2]>| {
            let slot = &mut index_of[j * dim + i];
            if *slot == u32::MAX {
                *slot = positions.len() as u32;
                positions.push([
                    origin.0 + i as f32 * cell,
                    level(i, j),
                    -(origin.1 + j as f32 * cell),
                ]);
                uvs.push([i as f32, j as f32]);
                colors.push(depth_tint(level(i, j) - ground(i, j)));
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
    (!indices.is_empty()).then(|| {
        let mut mesh = smooth_normal_mesh(positions, uvs, indices);
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
        mesh
    })
}

/// A translucent blue-green surface: the colour of the water seen at a slant, with a little
/// sheen. Reflection and foam are not part of it.
#[must_use]
pub fn water_material() -> StandardMaterial {
    StandardMaterial {
        // The shallow tint; the vertex colours of the surface darken it with depth and set the
        // opacity (`depth_tint`).
        base_color: Color::srgba(SHALLOW_RGB[0], SHALLOW_RGB[1], SHALLOW_RGB[2], 1.0),
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
mod tests;
