//! Unit tests for the path geometry (heading, arc length, point lookup).

use glam::DVec2;
use opts_traffic_lanes::geometry::PathGeometry;
use std::f64::consts::FRAC_PI_2;

#[test]
fn heading_zero_goes_north_and_ninety_east() {
    let n = PathGeometry::straight(DVec2::ZERO, 0.0, 10.0);
    assert!(n.end().abs_diff_eq(DVec2::new(0.0, 10.0), 1e-9));
    let e = PathGeometry::straight(DVec2::ZERO, FRAC_PI_2, 10.0);
    assert!(e.end().abs_diff_eq(DVec2::new(10.0, 0.0), 1e-9));
}

#[test]
fn positive_radius_turns_right() {
    // Quarter circle of radius 10 starting north: ends 10 east and 10 north, heading east.
    let arc = PathGeometry {
        start: DVec2::ZERO,
        heading: 0.0,
        radius: 10.0,
        length: 10.0 * FRAC_PI_2,
    };
    assert!(
        arc.end().abs_diff_eq(DVec2::new(10.0, 10.0), 1e-9),
        "{:?}",
        arc.end()
    );
    assert!((arc.end_heading() - FRAC_PI_2).abs() < 1e-9);
    let left = PathGeometry {
        radius: -10.0,
        ..arc
    };
    assert!(left.end().abs_diff_eq(DVec2::new(-10.0, 10.0), 1e-9));
}

#[test]
fn large_radii_arc_like_the_spline_mesh() {
    // A spline of radius 1100 m and 183 m length (Spandau has such): the end lies
    // L²/2R ≈ 15.2 m beside the straight line, exactly where the mesh formula puts it.
    let g = PathGeometry {
        start: DVec2::new(5.0, 7.0),
        heading: 0.3,
        radius: 1100.0,
        length: 183.0,
    };
    assert!(!g.is_straight());
    let h = g.end_heading();
    let mesh = g.start
        + DVec2::new(
            -1100.0 * (h.cos() - 0.3f64.cos()),
            1100.0 * (h.sin() - 0.3f64.sin()),
        );
    assert!(g.end().distance(mesh) < 0.01, "{:?} vs {mesh:?}", g.end());
    let straight = PathGeometry::straight(g.start, 0.3, 183.0).end();
    assert!((g.end().distance(straight) - 183.0 * 183.0 / 2200.0).abs() < 0.1);
    // Even a radius of 10⁷ m stays exact (no cancellation).
    let huge = PathGeometry {
        radius: 1.0e7,
        ..PathGeometry::straight(DVec2::ZERO, 0.0, 100.0)
    };
    assert!(huge.end().abs_diff_eq(DVec2::new(0.0005, 100.0), 1e-6));
}

#[test]
fn reversed_path_retraces_the_original() {
    let arc = PathGeometry {
        start: DVec2::new(3.0, 4.0),
        heading: 0.7,
        radius: -25.0,
        length: 18.0,
    };
    let back = arc.reversed();
    assert!(back.start.abs_diff_eq(arc.end(), 1e-9));
    assert!(back.end().abs_diff_eq(arc.start, 1e-9));
    let mid = arc.point_at(7.0);
    assert!(back.point_at(18.0 - 7.0).abs_diff_eq(mid, 1e-9));
}

#[test]
fn right_offset_on_a_right_curve_is_the_inner_lane() {
    let arc = PathGeometry {
        start: DVec2::ZERO,
        heading: 0.0,
        radius: 20.0,
        length: 10.0,
    };
    let inner = arc.offset(2.5);
    assert!((inner.radius - 17.5).abs() < 1e-12);
    // Same turning angle, shorter arc, start 2.5 m to the east.
    assert!(inner.start.abs_diff_eq(DVec2::new(2.5, 0.0), 1e-12));
    assert!((inner.length / inner.radius - arc.length / arc.radius).abs() < 1e-12);
}

#[test]
fn object_local_paths_rotate_clockwise() {
    // A path starting 5 m north of the object, object turned 90° clockwise: it starts 5 m east.
    let local = PathGeometry::straight(DVec2::new(0.0, 5.0), 0.0, 10.0);
    let world = local.to_world(DVec2::new(100.0, 100.0), FRAC_PI_2);
    assert!(
        world.start.abs_diff_eq(DVec2::new(105.0, 100.0), 1e-9),
        "{:?}",
        world.start
    );
    assert!((world.heading - FRAC_PI_2).abs() < 1e-12);
}
