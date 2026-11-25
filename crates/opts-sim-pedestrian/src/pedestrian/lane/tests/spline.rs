//! Unit tests of the lanes of placed splines.

use super::*;

#[test]
fn large_radius_splines_arc_like_their_mesh() {
    // r = 1100 m, 183 m long: the end lies L²/2R ≈ 15 m beside the straight line.
    let lanes = lanes_from_spline(&spline(0.0, 1100.0, 183.0), &profile(0.0), (0.0, 0.0));
    let end = lanes[0].end_point(true);
    let h = 183.0f64 / 1100.0;
    let exact = (10.0 + 1100.0 * (1.0 - h.cos()), 20.0 + 1100.0 * h.sin());
    assert!(
        (f64::from(end.x) - exact.0).abs() < 0.01 && (f64::from(end.y) - exact.1).abs() < 0.01,
        "{end:?} vs {exact:?}"
    );
}

#[test]
fn bike_lanes_of_a_composite_are_found_by_kind() {
    let mut p = profile(2.2);
    p.paths.push(SplinePath {
        path_type: 0,
        offset_x: 3.0,
        offset_z: 0.05,
        width: 0.1,
        direction: 2,
        extra: None,
    });
    let bikes = lanes_of_kind(
        &spline(0.0, 0.0, 20.0),
        &p,
        "x.sli",
        (0.0, 0.0),
        LaneKind::Bike,
    );
    assert_eq!(bikes.len(), 1);
    assert!((bikes[0].points[0].x - 13.0).abs() < 1e-4);
    // The 3 m road lane is a road, the footpath still a footpath.
    assert_eq!(
        lanes_of_kind(
            &spline(0.0, 0.0, 20.0),
            &p,
            "x.sli",
            (0.0, 0.0),
            LaneKind::Road
        )
        .len(),
        1
    );
    assert_eq!(
        lanes_from_spline(&spline(0.0, 0.0, 20.0), &p, (0.0, 0.0)).len(),
        1
    );
}

#[test]
fn right_turn_lane_is_shorter_and_ends_where_the_arc_does() {
    // Radius +30 turns right (clockwise): a path 2.2 m to the right is on the inside.
    let inst = spline(0.0, 30.0, 30.0 * std::f32::consts::FRAC_PI_2);
    let lanes = lanes_from_spline(&inst, &profile(2.2), (0.0, 0.0));
    let l = &lanes[0];
    let expected = (30.0 - 2.2) * std::f32::consts::FRAC_PI_2;
    assert!(
        (l.length() - expected).abs() < 0.1,
        "{} vs {expected}",
        l.length()
    );
    let end = l.end_point(true);
    // Centreline end after the quarter turn: (40, 50) heading east; the lane is 2.2 m to its
    // right (south).
    assert!(
        (end - Vec3::new(40.0, 47.8, 5.05)).length() < 0.1,
        "{end:?}"
    );
    let (_, h) = l.at(l.length());
    assert!((h - std::f32::consts::FRAC_PI_2).abs() < 0.05);
}

#[test]
fn position_at_follows_the_polyline() {
    let l = FootLane::new(
        vec![
            Vec3::ZERO,
            Vec3::new(0.0, 10.0, 0.0),
            Vec3::new(10.0, 10.0, 0.0),
        ],
        Travel::Forward,
        1.0,
    )
    .unwrap();
    let (p, h) = l.at(15.0);
    assert!((p - Vec3::new(5.0, 10.0, 0.0)).length() < 1e-4);
    assert!((h - std::f32::consts::FRAC_PI_2).abs() < 1e-4);
}
