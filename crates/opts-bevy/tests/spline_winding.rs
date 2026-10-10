//! Spline ribbon winding and road/terrain ordering, on synthetic data (no OMSI install needed).
//!
//! OMSI profiles list their points left to right. Seen from above, the ribbon must wind
//! counter-clockwise (front face = +Y) for any heading, curvature, slope or mirroring; a ribbon
//! laid out right to left is the "bottom" face and must face −Y. Terrain under a road must never
//! rise above the road surface.

use bevy::math::Vec3;
use bevy::render::mesh::{Indices, Mesh};
use opts_bevy::{
    carve_terrain_under_roads, extrude_spline_segment, road_footprint, TERRAIN_CELL_M,
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

#[test]
fn top_faces_of_a_road_face_up() {
    for inst in layouts() {
        let mesh = ribbon(vec![pt(-4.0, 0.1), pt(0.0, 0.1), pt(4.0, 0.1)], &inst);
        let normals = triangle_normals(&mesh);
        assert!(!normals.is_empty());
        for n in normals {
            assert!(
                n.y > 0.9,
                "top face points {n:?} for heading {} radius {} slope {} mirrored {}",
                inst.heading_deg,
                inst.radius_m,
                inst.gradient_start,
                inst.mirrored
            );
        }
    }
}

#[test]
fn bottom_faces_do_not_face_up() {
    for inst in layouts() {
        // The same carriageway listed right to left is the underside.
        let mesh = ribbon(vec![pt(4.0, 0.1), pt(0.0, 0.1), pt(-4.0, 0.1)], &inst);
        for n in triangle_normals(&mesh) {
            assert!(n.y < -0.9, "underside points {n:?}");
        }
    }
}

#[test]
fn curb_walls_face_away_from_the_road() {
    // Left-to-right profile with a raised right curb: carriageway, riser, curb top.
    let inst = instance(0.0, 0.0, 0.0, false);
    let mesh = ribbon(
        vec![pt(-4.0, 0.1), pt(4.0, 0.1), pt(4.0, 0.25), pt(4.15, 0.25)],
        &inst,
    );
    let normals = triangle_normals(&mesh);
    assert!(normals.iter().any(|n| n.x < -0.9), "riser faces the road");
    assert!(normals
        .iter()
        .filter(|n| n.y.abs() > 0.9)
        .all(|n| n.y > 0.0));
}

#[test]
fn vertex_normals_of_a_road_point_up() {
    let inst = instance(123.0, 70.0, 0.02, true);
    let mesh = ribbon(vec![pt(-4.0, 0.1), pt(4.0, 0.1)], &inst);
    let normals = mesh
        .attribute(Mesh::ATTRIBUTE_NORMAL)
        .and_then(|a| a.as_float3())
        .expect("normals");
    assert!(normals.iter().all(|n| n[1] > 0.9));
}

#[test]
fn terrain_never_covers_a_road() {
    // Flat terrain 0.25 m above the road datum (the Waldhofen curb-strip situation).
    let dim = 61;
    let terrain = MapTerrain {
        grid_cells: 60,
        heights: vec![10.25; dim * dim],
    };
    let inst = instance(0.0, 0.0, 0.0, false);
    let prof = profile(vec![pt(-4.0, 0.1), pt(4.0, 0.1)]);
    let footprint = road_footprint(&prof, &inst).expect("height profile deforms terrain");
    let carved = carve_terrain_under_roads(&terrain, &[footprint]);
    // Road runs north from (50, 50) for 40 m, 8 m wide, surface at 10.1.
    for j in 0..dim {
        for i in 0..dim {
            let (x, y) = (i as f32 * TERRAIN_CELL_M, j as f32 * TERRAIN_CELL_M);
            let h = carved.heights[j * dim + i];
            if (46.0..=54.0).contains(&x) && (50.0..=90.0).contains(&y) {
                assert!(h < 10.1, "terrain {h} covers the road at ({x}, {y})");
            }
            if !(30.0..=70.0).contains(&x) {
                assert_eq!(h, 10.25, "terrain away from the road must stay as authored");
            }
        }
    }
}

#[test]
fn splines_without_height_profile_leave_terrain_alone() {
    let mut prof = profile(vec![pt(-4.0, 0.1), pt(4.0, 0.1)]);
    prof.height_profiles.clear();
    assert!(road_footprint(&prof, &instance(0.0, 0.0, 0.0, false)).is_none());
}
