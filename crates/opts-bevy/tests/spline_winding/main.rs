//! Spline ribbon winding and road/terrain ordering, on synthetic data (no OMSI install needed).
//!
//! OMSI profiles list their points left to right. Seen from above, the ribbon must wind
//! counter-clockwise (front face = +Y) for any heading, curvature, slope or mirroring; a ribbon
//! laid out right to left is the "bottom" face and must face −Y. Terrain under a road must never
//! rise above the road surface.

use bevy::math::Vec3;
use bevy::mesh::{Indices, Mesh};
use opts_bevy::{
    carve_terrain, extrude_spline_segment, spline_ground_triangles, TileTerrain, TERRAIN_CELL_M,
};
use opts_core::formats::map::{MapSplineInstance, MapTerrain};
use opts_core::formats::scenery::sli::{
    SplineGraphicalProfile, SplineHeightProfile, SplineProfilePoint,
};
use opts_core::formats::scenery::SplineProfile;

fn pt(x: f32, z: f32) -> SplineProfilePoint {
    SplineProfilePoint {
        x,
        z,
        u: 0.0,
        v_slope: 1.0,
    }
}

fn profile(points: Vec<SplineProfilePoint>) -> SplineProfile {
    SplineProfile {
        graphical_profiles: vec![SplineGraphicalProfile {
            texture_index: 0,
            points,
        }],
        height_profiles: vec![SplineHeightProfile {
            x_start: -4.0,
            x_end: 4.0,
            z_start: 0.1,
            z_end: 0.1,
        }],
        ..Default::default()
    }
}

fn instance(heading: f32, radius: f32, slope: f32, mirrored: bool) -> MapSplineInstance {
    MapSplineInstance {
        flag: 0,
        sli_file: String::new(),
        id_code: 1,
        conn_id_1: 0,
        conn_id_2: 0,
        position: Vec3::new(50.0, 50.0, 10.0),
        heading_deg: heading,
        length_m: 40.0,
        radius_m: radius,
        gradient_start: slope,
        gradient_end: slope,
        has_height_profile: true,
        mirrored,
        raw_params: vec![],
    }
}

/// Geometric normals (right-handed, counter-clockwise front face) of every triangle.
fn triangle_normals(mesh: &Mesh) -> Vec<Vec3> {
    let pos = mesh
        .attribute(Mesh::ATTRIBUTE_POSITION)
        .and_then(|a| a.as_float3())
        .expect("positions");
    let idx: Vec<u32> = match mesh.indices().expect("indexed ribbon") {
        Indices::U32(v) => v.clone(),
        Indices::U16(v) => v.iter().map(|&i| u32::from(i)).collect(),
    };
    idx.chunks(3)
        .map(|t| {
            let p = |i: u32| Vec3::from(pos[i as usize]);
            let (a, b, c) = (p(t[0]), p(t[1]), p(t[2]));
            (b - a).cross(c - a).normalize_or_zero()
        })
        .filter(|n| *n != Vec3::ZERO)
        .collect()
}

fn ribbon(points: Vec<SplineProfilePoint>, inst: &MapSplineInstance) -> Mesh {
    extrude_spline_segment(&profile(points), inst, (0.0, 0.0))
        .into_iter()
        .next()
        .expect("one ribbon")
        .1
}

/// Straight, left/right arcs, slopes, all headings; mirrored or not.
fn layouts() -> Vec<MapSplineInstance> {
    let mut v = Vec::new();
    for mirrored in [false, true] {
        for heading in [0.0, 37.0, 90.0, 180.0, 271.0] {
            for (radius, slope) in [
                (0.0, 0.0),
                (60.0, 0.0),
                (-60.0, 0.0),
                (0.0, 0.05),
                (80.0, -0.06),
            ] {
                v.push(instance(heading, radius, slope, mirrored));
            }
        }
    }
    v
}

mod terrain;
mod winding;
