//! Unit tests of the lanes of placed scenery objects.

use super::*;

#[test]
fn object_paths_keep_the_straight_marker() {
    let path = |radius_m| ScoPath {
        start: Vec3::ZERO,
        heading_deg: 0.0,
        radius_m,
        length_m: 20.0,
        slope_start: 0.0,
        slope_end: 0.0,
        path_type: PATH_TYPE_PEDESTRIAN,
        width_m: 1.0,
        direction: 2,
        extra: None,
        traffic_light_index: None,
        approach_distance_m: None,
        switch_position: None,
    };
    let lanes = lanes_from_object(&[path(-999.99), path(500.0)], Vec3::ZERO, 0.0);
    assert!(lanes[0].end_point(true).x.abs() < 1e-4, "straight");
    assert!(lanes[1].end_point(true).x > 0.3, "a 500 m arc bends right");
}

#[test]
fn only_pedestrian_paths_become_lanes_right_of_travel() {
    // Heading north: right is east (+x).
    let lanes = lanes_from_spline(&spline(0.0, 0.0, 20.0), &profile(2.2), (0.0, 0.0));
    assert_eq!(lanes.len(), 1);
    let l = &lanes[0];
    assert_eq!(l.travel, Travel::Both);
    assert!((l.points[0] - Vec3::new(12.2, 20.0, 5.05)).length() < 1e-4);
    assert!((l.length() - 20.0).abs() < 1e-3);
    let (_, h) = l.at(5.0);
    assert!(h.abs() < 1e-4);
}

#[test]
fn object_paths_rotate_clockwise_with_the_object() {
    let path = ScoPath {
        start: Vec3::new(1.0, 0.0, 0.2),
        heading_deg: 0.0,
        radius_m: -999.99,
        length_m: 10.0,
        slope_start: 0.0,
        slope_end: 0.0,
        path_type: 1,
        width_m: 1.0,
        direction: 0,
        extra: None,
        traffic_light_index: None,
        approach_distance_m: None,
        switch_position: None,
    };
    // Object turned 90° clockwise: local north becomes east, local east becomes south.
    let lanes = lanes_from_object(&[path], Vec3::new(100.0, 200.0, 3.0), 90.0);
    let l = &lanes[0];
    assert!((l.points[0] - Vec3::new(100.0, 199.0, 3.2)).length() < 1e-3);
    assert!((l.end_point(true) - Vec3::new(110.0, 199.0, 3.2)).length() < 1e-3);
}
