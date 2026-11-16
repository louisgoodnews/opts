//! Train motion: advancing, retreating, poses behind the head and the branch choice.

use super::*;

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
