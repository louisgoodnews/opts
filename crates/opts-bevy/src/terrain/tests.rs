//! Unit tests of the terrain carve, mesh and height function.

use super::*;
use opts_core::formats::scenery::{SplineGraphicalProfile, SplineProfilePoint};

mod carve;
mod mesh;
mod street;

pub(super) fn flat(h: f32) -> TileTerrain {
    TileTerrain::new(
        MapTerrain {
            grid_cells: 60,
            heights: vec![h; 61 * 61],
        },
        5.0,
    )
}

fn profile(parts: &[&[(f32, f32)]]) -> SplineProfile {
    let mut p = SplineProfile::default();
    for pts in parts {
        p.graphical_profiles.push(SplineGraphicalProfile {
            texture_index: 0,
            points: pts
                .iter()
                .map(|&(x, z)| SplineProfilePoint {
                    x,
                    z,
                    u: 0.0,
                    v_slope: 0.0,
                })
                .collect(),
        });
    }
    p
}

/// A straight spline heading north from `(x, y)` at height `z`.
pub(super) fn spline(x: f32, y: f32, z: f32, length: f32) -> MapSplineInstance {
    MapSplineInstance {
        flag: 0,
        sli_file: String::new(),
        id_code: 1,
        conn_id_1: 0,
        conn_id_2: 0,
        position: Vec3::new(x, y, z),
        heading_deg: 0.0,
        length_m: length,
        radius_m: 0.0,
        gradient_start: 0.0,
        gradient_end: 0.0,
        has_height_profile: false,
        mirrored: false,
        raw_params: Vec::new(),
    }
}

fn h(t: &TileTerrain, i: usize, j: usize) -> f32 {
    t.grid.heights[j * 61 + i]
}
