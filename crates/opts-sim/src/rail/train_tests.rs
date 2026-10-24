//! Unit tests of `train.rs`.

use super::*;
use crate::pedestrian::{FootLane, Travel};

fn lane(a: Vec3, b: Vec3) -> FootLane {
    FootLane::new(vec![a, b], Travel::Both, 1.0).unwrap()
}

/// 100 m north, then a point: straight on (north) or a branch to the north-east.
fn net() -> PedestrianNet {
    let mut n = PedestrianNet::default();
    n.insert_tile(
        (0, 0),
        vec![
            lane(Vec3::ZERO, Vec3::new(0.0, 100.0, 0.0)),
            lane(Vec3::new(0.0, 100.0, 0.0), Vec3::new(0.0, 200.0, 0.0)),
            lane(Vec3::new(0.0, 100.0, 0.0), Vec3::new(50.0, 150.0, 0.0)),
        ],
    );
    n
}

#[test]
fn straight_branch_takes_the_through_route_and_the_trail_gives_the_rear() {
    let n = net();
    let mut t = Train::new(0, 90.0, 1);
    assert_eq!(t.advance(30.0, &n), Moved::Full);
    assert_eq!(t.lane, 1);
    let (p, h) = t.pose(&n).unwrap();
    assert!((p.y - 120.0).abs() < 1e-3 && h.abs() < 1e-4);
    // 25 m behind the head: 20 m on lane 1 … 5 m back on lane 0.
    let (q, _) = t.pose_behind(&n, 25.0).unwrap();
    assert!((q.y - 95.0).abs() < 1e-3, "{q:?}");
}

#[test]
fn the_next_points_are_found_with_their_ways() {
    let n = net();
    let t = Train::new(0, 70.0, 1);
    // 30 m to the point at the end of lane 0: straight on or to the right.
    let ahead = t.next_switch(&n, 100.0).expect("points ahead");
    assert!((ahead.distance_m - 30.0).abs() < 1e-3);
    assert_eq!(ahead.branches, vec![Branch::Straight, Branch::Right]);
    assert!(t.next_switch(&n, 20.0).is_none(), "out of range");
    // Past the point there is none.
    assert!(Train::new(1, 10.0, 1).next_switch(&n, 500.0).is_none());
}

#[test]
fn retreating_undoes_an_advance_and_the_trail_gives_the_rear_again() {
    let n = net();
    let mut t = Train::new(0, 90.0, 1);
    t.advance(30.0, &n);
    assert_eq!(t.lane, 1);
    // Back over the point onto lane 0, then a little further: the head faces north all along.
    assert_eq!(t.retreat(35.0, &n), Moved::Full);
    assert_eq!(t.lane, 0);
    let (p, h) = t.pose(&n).unwrap();
    assert!((p.y - 85.0).abs() < 1e-3 && h.abs() < 1e-4, "{p:?} {h}");
    // Forward again over the same point.
    t.advance(20.0, &n);
    assert_eq!(t.lane, 1);
    assert!((t.pose(&n).unwrap().0.y - 105.0).abs() < 1e-3);
}

#[test]
fn retreating_beyond_the_known_trail_follows_the_lane_behind() {
    let n = net();
    // Spawned on lane 1 (the trail is empty): backing up crosses to lane 0.
    let mut t = Train::new(1, 10.0, 1);
    assert_eq!(t.retreat(30.0, &n), Moved::Full);
    assert_eq!(t.lane, 0);
    let (p, h) = t.pose(&n).unwrap();
    assert!((p.y - 80.0).abs() < 1e-3 && h.abs() < 1e-4, "{p:?} {h}");
}

#[test]
fn retreating_off_the_beginning_of_the_track_ends_it() {
    let n = net();
    let mut t = Train::new(0, 5.0, 1);
    assert_eq!(t.retreat(10.0, &n), Moved::EndOfTrack);
    assert!(t.s.abs() < 1e-3);
}

#[test]
fn right_branch_takes_the_turn_and_poses_stay_continuous() {
    let n = net();
    let mut t = Train::new(0, 90.0, 1);
    t.branch = Branch::Right;
    t.advance(20.0, &n);
    assert_eq!(t.lane, 2);
    let (_, h) = t.pose(&n).unwrap();
    assert!((h - std::f32::consts::FRAC_PI_4).abs() < 0.05);
    let (q, _) = t.pose_behind(&n, 15.0).unwrap();
    assert!((q - Vec3::new(0.0, 95.0, 0.0)).length() < 0.5, "{q:?}");
}

#[test]
fn the_track_end_stops_the_head() {
    let n = net();
    let mut t = Train::new(1, 90.0, 1);
    assert_eq!(t.advance(50.0, &n), Moved::EndOfTrack);
    assert!((t.s - 100.0).abs() < 1e-3);
}

#[test]
fn rear_before_the_known_trail_extrapolates_along_the_tangent() {
    let n = net();
    let t = Train::new(0, 10.0, 1);
    let (q, _) = t.pose_behind(&n, 30.0).unwrap();
    assert!((q.y + 20.0).abs() < 1e-3, "{q:?}");
}

#[test]
fn a_route_decides_at_the_point_and_the_shortest_path_finds_it() {
    let n = net();
    assert_eq!(n.shortest_path(0, 1, 2, 1000.0), Some(vec![(2, 1)]));
    assert_eq!(n.shortest_path(0, 1, 1, 1000.0), Some(vec![(1, 1)]));
    assert_eq!(n.shortest_path(0, 1, 0, 1000.0), Some(Vec::new()));
    assert_eq!(n.shortest_path(0, 1, 2, 50.0), None, "beyond the limit");
    // Against the grain no lane leads to lane 1 from the start of lane 0 within reach of
    // the branch: walking backwards from lane 0 finds nothing.
    assert_eq!(n.shortest_path(0, -1, 2, 1000.0), None);
    let mut t = Train::new(0, 90.0, 1);
    t.route = [2].into();
    assert_eq!(t.advance(30.0, &n), Moved::Full);
    assert_eq!(t.lane, 2, "the route overrides the straight way");
    assert!(t.route.is_empty());
}

/// Lanes 1 (straight on) and 2 (to the north-east) belong to the points `K` in positions 0
/// and 1.
fn gated_net() -> PedestrianNet {
    use crate::pedestrian::Gate;
    let gate = |position| Gate {
        switch: 7,
        position,
    };
    let mut n = PedestrianNet::default();
    n.insert_tile(
        (0, 0),
        vec![
            lane(Vec3::ZERO, Vec3::new(0.0, 100.0, 0.0)),
            lane(Vec3::new(0.0, 100.0, 0.0), Vec3::new(0.0, 200.0, 0.0)).with_gate(gate(0)),
            lane(Vec3::new(0.0, 100.0, 0.0), Vec3::new(50.0, 150.0, 0.0)).with_gate(gate(1)),
        ],
    );
    n
}

#[test]
fn a_train_takes_the_open_way_whatever_it_wants() {
    let mut n = gated_net();
    let mut t = Train::new(0, 90.0, 1);
    t.branch = Branch::Right;
    n.set_switch_position(7, 0);
    t.advance(30.0, &n);
    assert_eq!(
        t.lane, 1,
        "points set to straight: the diverging lane is closed"
    );
    let mut t = Train::new(0, 90.0, 1);
    t.branch = Branch::Straight;
    n.set_switch_position(7, 1);
    t.advance(30.0, &n);
    assert_eq!(
        t.lane, 2,
        "points set to diverging: the straight lane is closed"
    );
}

#[test]
fn the_points_ahead_name_the_lane_the_train_wants_even_when_it_is_closed() {
    let mut n = gated_net();
    n.set_switch_position(7, 0);
    let mut t = Train::new(0, 50.0, 1);
    t.branch = Branch::Right;
    let ahead = t.next_switch(&n, 100.0).expect("points ahead");
    assert_eq!(ahead.wanted, 2);
    assert!(n.is_closed(ahead.wanted));
    t.branch = Branch::Straight;
    assert_eq!(t.next_switch(&n, 100.0).unwrap().wanted, 1);
}

#[test]
fn a_train_is_never_stranded_when_every_way_is_closed() {
    let mut n = gated_net();
    n.set_switch_position(7, 5);
    let mut t = Train::new(0, 90.0, 1);
    assert_eq!(t.advance(30.0, &n), Moved::Full);
    assert_eq!(t.lane, 1);
}
