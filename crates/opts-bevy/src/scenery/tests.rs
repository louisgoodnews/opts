//! Unit tests for `super` (`scenery.rs`).

use bevy::math::Vec3;
use bevy::mesh::Mesh;
use opts_core::formats::scenery::{SplineGraphicalProfile, SplineProfile, SplineProfilePoint};

use super::pose::height_offset;
use super::*;

#[test]
fn test_extrude_spline_profile() {
    let mut profile = SplineProfile::default();
    profile.graphical_profiles.push(SplineGraphicalProfile {
        texture_index: 0,
        points: vec![
            SplineProfilePoint {
                x: -2.0,
                z: 0.0,
                u: 0.0,
                v_slope: 0.2,
            },
            SplineProfilePoint {
                x: 2.0,
                z: 0.0,
                u: 1.0,
                v_slope: 0.2,
            },
        ],
    });

    let meshes = extrude_spline_profile(&profile, 10.0);
    assert_eq!(meshes.len(), 1);
    let (tex_idx, mesh) = &meshes[0];
    assert_eq!(*tex_idx, 0);

    // 2 points extruded produces 4 vertices and 2 triangles (6 indices)
    let positions = mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap();
    assert_eq!(positions.len(), 4);
    assert_eq!(mesh.indices().unwrap().len(), 6);
}

fn flat_profile() -> SplineProfile {
    let mut p = SplineProfile::default();
    p.graphical_profiles.push(SplineGraphicalProfile {
        texture_index: 0,
        points: vec![
            SplineProfilePoint {
                x: -2.0,
                z: 0.0,
                u: 0.0,
                v_slope: 0.2,
            },
            SplineProfilePoint {
                x: 2.0,
                z: 0.0,
                u: 1.0,
                v_slope: 0.2,
            },
        ],
    });
    p
}

fn inst(heading: f32, radius: f32) -> MapSplineInstance {
    MapSplineInstance {
        flag: 0,
        sli_file: String::new(),
        id_code: 1,
        conn_id_1: 0,
        conn_id_2: 0,
        position: Vec3::new(10.0, 20.0, 5.0),
        heading_deg: heading,
        length_m: 40.0,
        radius_m: radius,
        gradient_start: 0.0,
        gradient_end: 0.0,
        has_height_profile: false,
        mirrored: false,
        raw_params: Vec::new(),
    }
}

#[test]
fn straight_spline_heads_north_and_faces_up() {
    let meshes = extrude_spline_segment(&flat_profile(), &inst(0.0, 0.0), (300.0, 600.0));
    let mesh = &meshes[0].1;
    let pos: Vec<[f32; 3]> = mesh
        .attribute(Mesh::ATTRIBUTE_POSITION)
        .unwrap()
        .as_float3()
        .unwrap()
        .to_vec();
    // Start: east 310, up 5, north 620 → Bevy (310 ± 2, 5, -620).
    assert!((pos[0][0] - 308.0).abs() < 1e-3 && (pos[1][0] - 312.0).abs() < 1e-3);
    assert!((pos[0][1] - 5.0).abs() < 1e-3);
    assert!((pos[0][2] + 620.0).abs() < 1e-3);
    // End is 40 m further north (more negative Bevy z).
    assert!((pos.last().unwrap()[2] + 660.0).abs() < 1e-3);
    let normals = mesh
        .attribute(Mesh::ATTRIBUTE_NORMAL)
        .unwrap()
        .as_float3()
        .unwrap();
    assert!(normals.iter().all(|n| n[1] > 0.99));
}

#[test]
fn heading_90_heads_east() {
    let meshes = extrude_spline_segment(&flat_profile(), &inst(90.0, 0.0), (0.0, 0.0));
    let pos = meshes[0]
        .1
        .attribute(Mesh::ATTRIBUTE_POSITION)
        .unwrap()
        .as_float3()
        .unwrap();
    let last = pos.last().unwrap();
    assert!(
        last[0] > 10.0 + 39.0,
        "heading 90° must run towards +east, got {last:?}"
    );
}

#[test]
fn positive_radius_turns_right() {
    let meshes = extrude_spline_segment(&flat_profile(), &inst(0.0, 50.0), (0.0, 0.0));
    let pos = meshes[0]
        .1
        .attribute(Mesh::ATTRIBUTE_POSITION)
        .unwrap()
        .as_float3()
        .unwrap();
    let end_mid = (pos[pos.len() - 1][0] + pos[pos.len() - 2][0]) / 2.0;
    assert!(
        end_mid > 10.0,
        "positive radius curves towards east (right) when heading north"
    );
}

#[test]
fn plain_spline_rises_by_the_average_of_its_percent_gradients() {
    let mut inst = inst(0.0, 0.0);
    inst.length_m = 30.2;
    inst.gradient_start = 0.07;
    inst.gradient_end = 0.06;
    let z = height_offset(&inst, 30.2, 30.2);
    assert!((z - 1.963).abs() < 1e-3, "{z}");
}

#[test]
fn spline_h_reaches_its_explicit_height_with_a_smooth_ramp() {
    let mut inst = inst(0.0, 0.0);
    inst.length_m = 10.0;
    inst.has_height_profile = true;
    inst.raw_params = vec!["0".into(), "0".into(), "0.5".into()];
    assert!((height_offset(&inst, 10.0, 10.0) - 0.5).abs() < 1e-6);
    assert!(
        (height_offset(&inst, 5.0, 10.0) - 0.25).abs() < 1e-6,
        "S-curve midpoint"
    );
    assert!(height_offset(&inst, 1.0, 10.0) < 0.05, "starts flat");
}

mod mirror_tests;
mod pose_tests;
