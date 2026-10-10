//! Terrain height grids (`.map.terrain`) → Bevy meshes and height sampling.

use bevy::asset::Handle;
use bevy::pbr::StandardMaterial;
use bevy::render::mesh::Mesh;
use bevy::render::render_resource::Face;
use bevy::render::texture::Image;

use opts_core::formats::map::{MapSplineInstance, MapTerrain};
use opts_core::formats::scenery::SplineProfile;

use crate::mesh::smooth_normal_mesh;
use crate::scenery::spline_pose;

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

/// Ground a road-like spline covers: its centreline and the lowest surface height of its profile.
///
/// OMSI bakes a spline's `[heightprofile]` into the terrain, but the terrain grid is only 5 m wide
/// and its curb strips sit up to 0.15 m above the carriageway, so the raw grid pokes through the
/// road surface from above (the road then only shows from below).
#[derive(Debug, Clone, PartialEq)]
pub struct RoadFootprint {
    /// Centreline samples `(east, north, surface datum z)`, tile-local metres.
    pub centreline: Vec<[f32; 3]>,
    /// Half of the profile's total width (m).
    pub half_width: f32,
    /// Lowest profile point height relative to the centreline datum (m).
    pub min_surface_z: f32,
}

/// Footprint of a placed spline, or `None` when it does not deform the terrain (no height profile).
#[must_use]
pub fn road_footprint(profile: &SplineProfile, inst: &MapSplineInstance) -> Option<RoadFootprint> {
    if profile.height_profiles.is_empty() {
        return None;
    }
    let points = || profile.graphical_profiles.iter().flat_map(|g| &g.points);
    let half_width = points().map(|p| p.x.abs()).fold(0.0_f32, f32::max);
    let min_surface_z = points().map(|p| p.z).fold(f32::INFINITY, f32::min);
    if half_width <= 0.0 || !min_surface_z.is_finite() {
        return None;
    }
    let length = inst.length_m.max(0.01);
    let steps = ((length / 2.0).ceil() as usize).clamp(1, 400);
    let centreline = (0..=steps)
        .map(|k| {
            let p = spline_pose(inst, length * k as f32 / steps as f32);
            [p.x, p.y, p.z]
        })
        .collect();
    Some(RoadFootprint {
        centreline,
        half_width,
        min_surface_z,
    })
}

/// Lowers terrain vertices under road surfaces so the terrain never covers them.
///
/// A vertex within `half_width + ½ cell` of a road centreline is capped at the road's lowest
/// surface height minus a small clearance; vertices elsewhere and heights already below the road
/// stay untouched.
#[must_use]
pub fn carve_terrain_under_roads(terrain: &MapTerrain, roads: &[RoadFootprint]) -> MapTerrain {
    const CLEARANCE_M: f32 = 0.03;
    let dim = terrain.grid_cells as usize + 1;
    let mut out = terrain.clone();
    if out.heights.len() < dim * dim {
        return out;
    }
    for road in roads {
        let reach = road.half_width + TERRAIN_CELL_M * 0.5;
        for seg in road.centreline.windows(2) {
            let (a, b) = (seg[0], seg[1]);
            let (min_x, max_x) = (a[0].min(b[0]) - reach, a[0].max(b[0]) + reach);
            let (min_y, max_y) = (a[1].min(b[1]) - reach, a[1].max(b[1]) + reach);
            let i0 = ((min_x / TERRAIN_CELL_M).floor().max(0.0)) as usize;
            let j0 = ((min_y / TERRAIN_CELL_M).floor().max(0.0)) as usize;
            let i1 = (((max_x / TERRAIN_CELL_M).ceil()).max(0.0) as usize).min(dim - 1);
            let j1 = (((max_y / TERRAIN_CELL_M).ceil()).max(0.0) as usize).min(dim - 1);
            let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
            let len2 = (dx * dx + dy * dy).max(1e-9);
            for j in j0..=j1 {
                for i in i0..=i1 {
                    let (vx, vy) = (i as f32 * TERRAIN_CELL_M, j as f32 * TERRAIN_CELL_M);
                    let t = (((vx - a[0]) * dx + (vy - a[1]) * dy) / len2).clamp(0.0, 1.0);
                    let (px, py) = (a[0] + t * dx, a[1] + t * dy);
                    if (vx - px).hypot(vy - py) > reach {
                        continue;
                    }
                    let ceiling = a[2] + t * (b[2] - a[2]) + road.min_surface_z - CLEARANCE_M;
                    let h = &mut out.heights[j * dim + i];
                    *h = h.min(ceiling);
                }
            }
        }
    }
    out
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
