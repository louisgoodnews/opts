//! Degenerate-input tests for the path geometry (zero and negative lengths).

use glam::DVec2;
use opts_traffic_lanes::geometry::PathGeometry;

#[test]
fn negative_or_zero_lengths_do_not_panic() {
    let g = PathGeometry::straight(DVec2::new(1.0, 2.0), 0.3, -9.11);
    assert_eq!(g.point_at(5.0), g.start);
    assert_eq!(
        PathGeometry::straight(DVec2::ZERO, 0.0, 0.0).end(),
        DVec2::ZERO
    );
}
