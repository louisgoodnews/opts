//! Mirror tests: a mirrored spline lays its profile out to the left of the centreline.

use bevy::math::Vec3;
use bevy::mesh::Mesh;
use opts_core::formats::map::MapSplineInstance;
use opts_core::formats::scenery::SplineProfile;
use opts_core::formats::scenery::sli::{SplineGraphicalProfile, SplineProfilePoint};

use super::super::extrude::extrude_spline_segment;

fn x_extent(mirrored: bool) -> (f32, f32) {
    let pt = |x: f32| SplineProfilePoint {
        x,
        z: 0.0,
        u: 0.0,
        v_slope: 1.0,
    };
    let profile = SplineProfile {
        graphical_profiles: vec![SplineGraphicalProfile {
            texture_index: 0,
            points: vec![pt(0.0), pt(4.0)],
        }],
        ..Default::default()
    };
    let mut inst = pose_tests_instance();
    inst.heading_deg = 0.0; // north: right is +x
    inst.position = Vec3::ZERO;
    inst.mirrored = mirrored;
    let meshes = extrude_spline_segment(&profile, &inst, (0.0, 0.0));
    let positions = meshes[0]
        .1
        .attribute(Mesh::ATTRIBUTE_POSITION)
        .and_then(|a| a.as_float3().map(<[_]>::to_vec))
        .unwrap();
    let xs: Vec<f32> = positions.iter().map(|p| p[0]).collect();
    (
        xs.iter().copied().fold(f32::MAX, f32::min),
        xs.iter().copied().fold(f32::MIN, f32::max),
    )
}

fn pose_tests_instance() -> MapSplineInstance {
    MapSplineInstance {
        flag: 0,
        sli_file: String::new(),
        id_code: 1,
        conn_id_1: 0,
        conn_id_2: 0,
        position: Vec3::ZERO,
        heading_deg: 0.0,
        length_m: 10.0,
        radius_m: 0.0,
        gradient_start: 0.0,
        gradient_end: 0.0,
        has_height_profile: false,
        mirrored: false,
        raw_params: vec![],
    }
}

#[test]
fn mirrored_spline_flips_the_profile_to_the_left() {
    assert_eq!(x_extent(false), (0.0, 4.0));
    assert_eq!(x_extent(true), (-4.0, 0.0));
}
