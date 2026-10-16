//! Terrain height grids (`.map.terrain`) → Bevy meshes and height sampling.

use bevy::asset::Handle;
use bevy::image::Image;
use bevy::mesh::Mesh;
use bevy::pbr::StandardMaterial;
use bevy::render::render_resource::Face;

use opts_core::formats::map::{MapSplineInstance, MapTerrain};
use opts_core::formats::scenery::SplineProfile;

use crate::mesh::smooth_normal_mesh;
use crate::scenery::spline_pose;

/// Default side length of a map tile in metres (`docs/src/formats/map/tiles.md` Q-M01). Maps with
/// `[worldcoordinates]` use larger tiles (Berlin-Spandau 372 m, Wuppertal 639 382.8 m): see
/// [`tile_size`].
pub const TILE_SIZE_M: f32 = 300.0;

/// Default terrain grid cell size (60 cells per default tile); see [`terrain_cell`].
pub const TERRAIN_CELL_M: f32 = TILE_SIZE_M / 60.0;

static TILE_SIZE_BITS: std::sync::atomic::AtomicU32 =
    std::sync::atomic::AtomicU32::new(TILE_SIZE_M.to_bits());

/// Side length of a tile of the map being played (set once per session by [`set_tile_size`]).
#[must_use]
pub fn tile_size() -> f32 {
    f32::from_bits(TILE_SIZE_BITS.load(std::sync::atomic::Ordering::Relaxed))
}

/// Sets the tile size of the current map.
pub fn set_tile_size(metres: f32) {
    TILE_SIZE_BITS.store(metres.to_bits(), std::sync::atomic::Ordering::Relaxed);
}

/// Terrain cell size of the current map (a tile has 60 × 60 cells).
#[must_use]
pub fn terrain_cell() -> f32 {
    tile_size() / 60.0
}

/// Bilinearly samples a tile's height grid at tile-local `(x, y)` metres.
#[must_use]
pub fn sample_terrain(terrain: &MapTerrain, x: f32, y: f32) -> f32 {
    let dim = terrain.grid_cells as usize + 1;
    if dim < 2 || terrain.heights.len() < dim * dim {
        return 0.0;
    }
    let max = terrain.grid_cells as f32 - 0.0001;
    let gx = (x / terrain_cell()).clamp(0.0, max);
    let gy = (y / terrain_cell()).clamp(0.0, max);
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
                origin.0 + i as f32 * terrain_cell(),
                h,
                -(origin.1 + j as f32 * terrain_cell()),
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
    /// Terrain further than this above the surface is left alone (a cutting or tunnel roof, not a
    /// grid that merely covers the surface): infinite for roads with a height profile.
    pub max_cover: f32,
}

/// Narrowest flat spline (m) that is treated as a ground surface the terrain must not cover.
const FLAT_SURFACE_MIN_WIDTH_M: f32 = 2.0;
/// How far above a flat surface the terrain may lie and still be lowered onto it (m).
const FLAT_SURFACE_MAX_COVER_M: f32 = 0.4;

/// Footprint of a placed spline, or `None` when it does not deform the terrain (no height profile).
#[must_use]
pub fn road_footprint(profile: &SplineProfile, inst: &MapSplineInstance) -> Option<RoadFootprint> {
    // Flat surfaces without a height profile (pavements, plazas, bus bays laid on the ground) are
    // authored a few centimetres below the terrain grid and would be covered by it.
    let flat_surface = !profile.only_editor && profile.flat_width() >= FLAT_SURFACE_MIN_WIDTH_M;
    if profile.height_profiles.is_empty() && !flat_surface {
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
        max_cover: if profile.height_profiles.is_empty() {
            FLAT_SURFACE_MAX_COVER_M
        } else {
            f32::INFINITY
        },
    })
}

/// The footprint of a spline placed on a neighbouring tile, moved into this tile's frame, or
/// `None` when it does not come within reach of this tile's grid.
///
/// `offset` is the neighbour's south-west corner relative to this tile's (`(dx·tile, dy·tile)`).
/// Splines are stored on the tile they start in but run on across the border; a tile carved only
/// under its own splines covered the part of a road that lies on the next tile (Neuenbreid: 286 m
/// of road surface under the neighbour's terrain, 7 m within the splines' own tiles).
#[must_use]
pub fn footprint_from_neighbour(road: &RoadFootprint, offset: (f32, f32)) -> Option<RoadFootprint> {
    let reach = road.half_width + terrain_cell();
    let size = tile_size();
    let centreline: Vec<[f32; 3]> = road
        .centreline
        .iter()
        .map(|p| [p[0] + offset.0, p[1] + offset.1, p[2]])
        .collect();
    let near = centreline.iter().any(|p| {
        (-reach..=size + reach).contains(&p[0]) && (-reach..=size + reach).contains(&p[1])
    });
    near.then(|| RoadFootprint {
        centreline,
        ..road.clone()
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
        let reach = road.half_width + terrain_cell() * 0.5;
        for seg in road.centreline.windows(2) {
            let (a, b) = (seg[0], seg[1]);
            let (min_x, max_x) = (a[0].min(b[0]) - reach, a[0].max(b[0]) + reach);
            let (min_y, max_y) = (a[1].min(b[1]) - reach, a[1].max(b[1]) + reach);
            let i0 = ((min_x / terrain_cell()).floor().max(0.0)) as usize;
            let j0 = ((min_y / terrain_cell()).floor().max(0.0)) as usize;
            let i1 = (((max_x / terrain_cell()).ceil()).max(0.0) as usize).min(dim - 1);
            let j1 = (((max_y / terrain_cell()).ceil()).max(0.0) as usize).min(dim - 1);
            let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
            let len2 = (dx * dx + dy * dy).max(1e-9);
            for j in j0..=j1 {
                for i in i0..=i1 {
                    let (vx, vy) = (i as f32 * terrain_cell(), j as f32 * terrain_cell());
                    let t = (((vx - a[0]) * dx + (vy - a[1]) * dy) / len2).clamp(0.0, 1.0);
                    let (px, py) = (a[0] + t * dx, a[1] + t * dy);
                    if (vx - px).hypot(vy - py) > reach {
                        continue;
                    }
                    let ceiling = a[2] + t * (b[2] - a[2]) + road.min_surface_z - CLEARANCE_M;
                    let h = &mut out.heights[j * dim + i];
                    if *h - ceiling <= road.max_cover {
                        *h = h.min(ceiling);
                    }
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

    #[test]
    fn a_road_from_the_neighbour_tile_is_carved_too() {
        set_tile_size(300.0);
        let dim = 61;
        let flat = MapTerrain {
            grid_cells: 60,
            heights: vec![1.0; dim * dim],
        };
        // A road on the western neighbour, running east from x = 280 to x = 340 there: its last
        // 40 m lie on this tile (x = -20 .. 40 here), 0.5 m below the terrain.
        let road = RoadFootprint {
            centreline: (0..=12)
                .map(|k| [280.0 + 5.0 * k as f32, 150.0, 0.5])
                .collect(),
            half_width: 3.0,
            min_surface_z: 0.0,
            max_cover: f32::INFINITY,
        };
        let here = footprint_from_neighbour(&road, (-300.0, 0.0)).expect("reaches this tile");
        let carved = carve_terrain_under_roads(&flat, &[here]);
        assert!(sample_terrain(&carved, 20.0, 150.0) < 0.5);
        assert!((sample_terrain(&carved, 100.0, 150.0) - 1.0).abs() < 1e-4);
        // A road that stays on the neighbour does not reach this tile.
        let far = RoadFootprint {
            centreline: vec![[100.0, 150.0, 0.5], [200.0, 150.0, 0.5]],
            ..road
        };
        assert!(footprint_from_neighbour(&far, (-300.0, 0.0)).is_none());
    }
}
