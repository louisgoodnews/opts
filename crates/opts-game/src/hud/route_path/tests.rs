//! Tests of the route path: the joins between stops with and without lanes.

use super::*;

#[test]
fn without_lanes_the_stops_are_joined_by_straight_connectors() {
    let stops = [
        RouteStop {
            pos: DVec2::new(0.0, 0.0),
            place: None,
        },
        RouteStop {
            pos: DVec2::new(30.0, 40.0),
            place: None,
        },
    ];
    let roads = path_between(&LaneGraph::default(), &stops);
    assert_eq!(roads.len(), 1);
    assert!((roads[0].geometry.length - 50.0).abs() < 1e-9);
    let end = roads[0].geometry.point_at(50.0);
    assert!(end.distance(DVec2::new(30.0, 40.0)) < 1e-6);
}
