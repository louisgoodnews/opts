//! Centreline pose tests: a straight spline climbs, and a right turn ends south of its start.

use bevy::math::Vec3;
use opts_core::formats::map::MapSplineInstance;

use super::super::pose::spline_pose;

fn straight() -> MapSplineInstance {
    MapSplineInstance {
        flag: 0,
        sli_file: String::new(),
        id_code: 1,
        conn_id_1: 0,
        conn_id_2: 0,
        position: Vec3::new(10.0, 20.0, 5.0),
        heading_deg: 90.0,
        length_m: 100.0,
        radius_m: 0.0,
        gradient_start: 0.02,
        gradient_end: 0.02,
        has_height_profile: false,
        mirrored: false,
        raw_params: vec![],
    }
}

#[test]
fn straight_pose_heads_east_and_climbs() {
    let p = spline_pose(&straight(), 50.0);
    assert!((p.x - 60.0).abs() < 1e-3 && (p.y - 20.0).abs() < 1e-3);
    assert!((p.z - 6.0).abs() < 1e-4);
}

#[test]
fn right_turn_arc_ends_south_of_east_start() {
    let mut inst = straight();
    inst.radius_m = 100.0;
    inst.length_m = 200.0;
    let p = spline_pose(&inst, 100.0 * std::f32::consts::FRAC_PI_2);
    // Heading east, turning right by 90° ends heading south, 100 m east and 100 m south.
    assert!((p.x - 110.0).abs() < 0.05 && (p.y - -80.0).abs() < 0.05);
}
