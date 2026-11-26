//! Unit tests of the zone overlay: the box geometry and the toggle state.

use bevy::math::{Vec2, Vec3};

use opts_spatial_zones::model::{SpatialZone, ZoneId, ZoneKind};
use opts_spatial_zones::overlay::*;

fn tunnel() -> SpatialZone {
    SpatialZone {
        id: ZoneId(7),
        kind: ZoneKind::Tunnel,
        min: Vec2::new(-20.0, 5.0),
        max: Vec2::new(30.0, 15.0),
    }
}

#[test]
fn the_overlay_is_off_by_default_and_toggles() {
    let mut overlay = ZoneOverlay::default();
    assert!(!overlay.on());
    overlay.toggle();
    assert!(overlay.on());
    overlay.toggle();
    assert!(!overlay.on());
}

#[test]
fn ground_corners_sit_on_the_box_corners_at_the_given_height() {
    let z = tunnel();
    let c = ground_corners(z.min, z.max, 2.5);
    assert_eq!(c[0], Vec3::new(-20.0, 2.5, 5.0));
    assert_eq!(c[1], Vec3::new(30.0, 2.5, 5.0));
    assert_eq!(c[2], Vec3::new(30.0, 2.5, 15.0));
    assert_eq!(c[3], Vec3::new(-20.0, 2.5, 15.0));
}

#[test]
fn every_corner_is_a_ground_point_the_sampler_counts_as_inside() {
    let z = tunnel();
    for c in ground_corners(z.min, z.max, ZONE_BOTTOM_M) {
        assert!(z.contains(Vec2::new(c.x, c.z)), "{c:?} is outside the box");
    }
}

#[test]
fn the_box_has_twelve_axis_aligned_edges_joining_its_corners() {
    let z = tunnel();
    let edges = box_edges(&z);
    for (a, b) in edges {
        let axis_aligned = (a.x - b.x).abs() < f32::EPSILON
            || (a.y - b.y).abs() < f32::EPSILON
            || (a.z - b.z).abs() < f32::EPSILON;
        assert!(axis_aligned, "{a:?} -> {b:?} is diagonal");
        assert!(a != b, "a degenerate edge at {a:?}");
    }
    // Four verticals, each as long as the zone is tall.
    let verticals = edges
        .iter()
        .skip(8)
        .filter(|(a, b)| a.x == b.x && a.z == b.z);
    assert_eq!(verticals.clone().count(), 4);
    for (a, b) in verticals {
        assert!((b.y - a.y - (ZONE_TOP_M - ZONE_BOTTOM_M)).abs() < 1e-6);
    }
}

#[test]
fn the_floor_and_roof_edges_cover_the_perimeter_once() {
    let z = tunnel();
    let edges = box_edges(&z);
    let perimeter: f32 = edges[..4].iter().map(|(a, b)| a.distance(*b)).sum();
    // 50 m along x twice and 10 m along z twice.
    assert!((perimeter - 120.0).abs() < 1e-4);
    let roof_total: f32 = edges[4..8].iter().map(|(a, b)| a.distance(*b)).sum();
    assert!((roof_total - perimeter).abs() < 1e-4);
}
