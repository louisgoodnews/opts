//! Unit tests of `graph.rs` (moved out of the file to keep it under the 500-line limit, refactoring cycle 1).

use super::*;

fn road(x: f64, y: f64, heading: f64, len: f64) -> LaneSegment {
    LaneSegment::straight(DVec2::new(x, y), heading, len, LaneKind::Road)
}

#[test]
fn end_to_start_joins_and_gaps_do_not() {
    let g = LaneGraph::build(
        &[
            road(0.0, 0.0, 0.0, 50.0),
            road(0.0, 50.2, 0.0, 50.0),
            road(0.0, 120.0, 0.0, 10.0),
        ],
        &ConnectParams::default(),
    );
    assert_eq!(g.successors(LaneId(0)), &[LaneId(1)]);
    assert!(g.successors(LaneId(1)).is_empty());
}

#[test]
fn heading_must_match() {
    // Second lane starts at the right place but points east.
    let g = LaneGraph::build(
        &[
            road(0.0, 0.0, 0.0, 50.0),
            road(0.0, 50.0, std::f64::consts::FRAC_PI_2, 20.0),
        ],
        &ConnectParams::default(),
    );
    assert!(g.successors(LaneId(0)).is_empty());
}

#[test]
fn kinds_do_not_mix() {
    let mut ped = road(0.0, 50.0, 0.0, 10.0);
    ped.kind = LaneKind::Pedestrian;
    let g = LaneGraph::build(&[road(0.0, 0.0, 0.0, 50.0), ped], &ConnectParams::default());
    assert!(g.successors(LaneId(0)).is_empty());
}

#[test]
fn directions_expand_to_directed_lanes() {
    let mut both = road(0.0, 0.0, 0.0, 30.0);
    both.direction = Direction::Both;
    let mut rev = road(0.0, 100.0, 0.0, 30.0);
    rev.direction = Direction::Reverse;
    let g = LaneGraph::build(&[both, rev], &ConnectParams::default());
    assert_eq!(g.len(), 3);
    // The reversed lane starts at the original end and heads south.
    let l = g.lane(LaneId(2)).unwrap();
    assert!(l.geometry.start.abs_diff_eq(DVec2::new(0.0, 130.0), 1e-9));
    assert!(angle_diff(l.geometry.heading, std::f64::consts::PI) < 1e-9);
}

#[test]
fn vehicle_type_rules_are_a_whitelist() {
    // Project lead 2026-10-19: a type rule means only those types are allowed.
    use crate::idm::VehicleClass::{Bus, Car, Truck};
    let none = LaneRules::default();
    assert!([Car, Truck, Bus].iter().all(|c| none.allows(*c)));
    let bus_lane = LaneRules { bus: true, ..none };
    assert_eq!(
        [
            bus_lane.allows(Car),
            bus_lane.allows(Truck),
            bus_lane.allows(Bus)
        ],
        [false, false, true]
    );
    let truck_lane = LaneRules {
        trucks: true,
        ..none
    };
    assert_eq!(
        [
            truck_lane.allows(Car),
            truck_lane.allows(Truck),
            truck_lane.allows(Bus)
        ],
        [false, true, false]
    );
    let both = LaneRules {
        bus: true,
        trucks: true,
        ..none
    };
    assert_eq!(
        [both.allows(Car), both.allows(Truck), both.allows(Bus)],
        [false, true, true]
    );
    let no_cars = LaneRules {
        no_cars: true,
        ..none
    };
    assert_eq!(
        [
            no_cars.allows(Car),
            no_cars.allows(Truck),
            no_cars.allows(Bus)
        ],
        [false, true, true]
    );
}

#[test]
fn elevation_follows_the_gradients_and_reverses() {
    let e = Elevation {
        z0: 10.0,
        grad_start: 0.02,
        grad_end: 0.06,
        rise: None,
    };
    // rise = length · (g0 + g1) / 2 = 100 · 0.04 = 4 m.
    assert!((e.at(100.0, 100.0) - 14.0).abs() < 1e-9);
    assert!((e.at(50.0, 100.0) - (10.0 + 50.0 * 0.03)).abs() < 1e-9);
    let back = e.reversed(100.0);
    assert!((back.z0 - 14.0).abs() < 1e-9);
    assert!((back.at(100.0, 100.0) - 10.0).abs() < 1e-9);
    assert!((back.at(50.0, 100.0) - e.at(50.0, 100.0)).abs() < 1e-9);
}

#[test]
fn spline_h_elevation_is_the_mesh_ramp_and_reverses() {
    // Flat ends, 2 m up over 40 m: the S-shaped ramp of the mesh, not a linear gradient.
    let e = Elevation {
        z0: 5.0,
        grad_start: 0.0,
        grad_end: 0.0,
        rise: Some(2.0),
    };
    assert!((e.at(40.0, 40.0) - 7.0).abs() < 1e-9);
    assert!((e.at(20.0, 40.0) - 6.0).abs() < 1e-9);
    assert!(e.at(4.0, 40.0) - 5.0 < 0.06, "starts flat");
    let back = e.reversed(40.0);
    for s in [0.0, 7.0, 20.0, 33.0, 40.0] {
        assert!((back.at(40.0 - s, 40.0) - e.at(s, 40.0)).abs() < 1e-9);
    }
}

#[test]
fn keys_survive_a_rebuild_with_other_segments() {
    let mut a = road(0.0, 0.0, 0.0, 30.0);
    a.source = 5;
    a.direction = Direction::Both;
    let mut b = road(0.0, 100.0, 0.0, 30.0);
    b.source = 6;
    let g1 = LaneGraph::build(&[a.clone(), b.clone()], &ConnectParams::default());
    let g2 = LaneGraph::build(&[b, a], &ConnectParams::default());
    assert_eq!(g1.lane_by_key((5, 1)), Some(LaneId(1)));
    assert_eq!(g2.lane_by_key((5, 1)), Some(LaneId(2)));
    assert_eq!(g2.lane_by_key((9, 0)), None);
}

/// Lanes are drawn in proportion to their weight; a lane weighing 0 never.
#[test]
fn weighted_spots_follow_the_weights() {
    let g = LaneGraph::build(
        &[
            road(0.0, 0.0, 0.0, 200.0),
            road(10.0, 0.0, 0.0, 200.0),
            road(20.0, 0.0, 0.0, 200.0),
        ],
        &ConnectParams::default(),
    );
    let centre = DVec2::new(10.0, 100.0);
    let x_of = |id: LaneId| g.lane(id).unwrap().geometry.start.x;
    let mut rng = Rng::new(3);
    let mut counts = [0usize; 3];
    for _ in 0..3000 {
        let (id, _) = g
            .weighted_spot(centre, 0.0, 150.0, &mut rng, |l| match l.geometry.start.x {
                x if x < 5.0 => 3.0,
                x if x < 15.0 => 1.0,
                _ => 0.0,
            })
            .unwrap();
        counts[(x_of(id) / 10.0).round() as usize] += 1;
    }
    assert_eq!(counts[2], 0);
    let share = counts[0] as f64 / (counts[0] + counts[1]) as f64;
    assert!((share - 0.75).abs() < 0.04, "share {share}");
    assert!(g
        .weighted_spot(centre, 0.0, 150.0, &mut rng, |_| 0.0)
        .is_none());
}

#[test]
fn random_spots_lie_in_the_requested_band() {
    let g = LaneGraph::build(&[road(0.0, 0.0, 0.0, 400.0)], &ConnectParams::default());
    let mut rng = Rng::new(9);
    let mut found = 0;
    for _ in 0..50 {
        if let Some((id, s)) =
            g.random_spot(DVec2::new(0.0, 200.0), 50.0, 120.0, &mut rng, |_| true)
        {
            let d = g
                .lane(id)
                .unwrap()
                .geometry
                .point_at(s)
                .distance(DVec2::new(0.0, 200.0));
            assert!((50.0..=120.0).contains(&d));
            found += 1;
        }
    }
    assert!(found > 20, "only {found} spots");
    assert!(g
        .random_spot(DVec2::new(0.0, 200.0), 50.0, 120.0, &mut rng, |_| false)
        .is_none());
}

#[test]
fn shortest_route_picks_the_shorter_branch() {
    // A fork after lane 0: lane 1 is long, lane 2 short, both rejoin in lane 3 (straight
    // stubs are enough to test the search).
    let mut long = road(0.0, 50.0, 0.0, 200.0);
    long.source = 1;
    let short = road(0.0, 50.0, 0.0, 20.0);
    let end_long = road(0.0, 250.0, 0.0, 10.0);
    let end_short = road(0.0, 70.0, 0.0, 10.0);
    let g = LaneGraph::build(
        &[road(0.0, 0.0, 0.0, 50.0), long, short, end_long, end_short],
        &ConnectParams::default(),
    );
    assert_eq!(g.successors(LaneId(0)).len(), 2);
    let route = g.shortest_route(LaneId(0), LaneId(4)).unwrap();
    assert_eq!(route, [LaneId(0), LaneId(2), LaneId(4)]);
    assert!(g.shortest_route(LaneId(4), LaneId(0)).is_none());
}
