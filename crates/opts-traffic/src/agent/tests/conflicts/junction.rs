//! Conflicts at crossings and merges: bodies never touch, and an unattended intersection is not blocked.

use super::*;

#[test]
fn crossing_flows_take_turns_without_touching() {
    use std::f64::consts::FRAC_PI_2;
    // North-bound and east-bound lanes cross at (0, 100); both vehicles arrive together.
    let north = straight(0.0, 200.0);
    let east = LaneSegment::straight(DVec2::new(-100.0, 100.0), FRAC_PI_2, 200.0, LaneKind::Road);
    let mut w = world(&[north, east]);
    let a = w
        .spawn(LaneId(0), 60.0, 12.0, IdmParams::default())
        .unwrap();
    let b = w
        .spawn(LaneId(1), 60.0, 12.0, IdmParams::default())
        .unwrap();
    let min = min_body_distance(&mut w, 400);
    assert!(min > 1.4, "bodies came within {min} m");
    // Both got through (and left at the dead ends) without a deadlock.
    assert!(w.agent(a).is_none() && w.agent(b).is_none());
}

#[test]
fn without_conflict_handling_the_same_crossing_would_collide() {
    use std::f64::consts::FRAC_PI_2;
    // Negative control for the test above: with conflicts ignored the bodies overlap.
    let north = straight(0.0, 200.0);
    let east = LaneSegment::straight(DVec2::new(-100.0, 100.0), FRAC_PI_2, 200.0, LaneKind::Road);
    let mut w = world(&[north, east]);
    w.spawn(LaneId(0), 60.0, 12.0, IdmParams::default())
        .unwrap();
    w.spawn(LaneId(1), 60.0, 12.0, IdmParams::default())
        .unwrap();
    for a in &mut w.agents {
        a.ignore_conflicts_until = f64::INFINITY;
    }
    assert!(min_body_distance(&mut w, 400) < 1.0);
}

#[test]
fn merging_lanes_take_turns() {
    let mut w = world(&[
        straight(0.0, 60.0),
        straight(60.0, 100.0),
        LaneSegment::straight(DVec2::new(-0.2, 10.0), 0.0, 50.2, LaneKind::Road),
    ]);
    w.spawn(LaneId(0), 30.0, 10.0, IdmParams::default())
        .unwrap();
    w.spawn(LaneId(2), 30.0, 10.0, IdmParams::default())
        .unwrap();
    let min = min_body_distance(&mut w, 400);
    assert!(min > 1.4, "bodies came within {min} m");
    assert!(w.agents().is_empty());
}

#[test]
fn parallel_traffic_is_not_held_up() {
    let neighbour = LaneSegment::straight(DVec2::new(3.5, 0.0), 0.0, 400.0, LaneKind::Road);
    let mut w = world(&[straight(0.0, 400.0), neighbour]);
    let a = w.spawn(LaneId(0), 0.0, 12.0, IdmParams::default()).unwrap();
    w.spawn(LaneId(1), 0.0, 12.0, IdmParams::default()).unwrap();
    for _ in 0..100 {
        w.step(0.1);
    }
    assert!(w.agent(a).unwrap().speed > 10.0);
}

#[test]
fn a_stuck_vehicle_goes_anyway_after_the_deadlock_timeout() {
    use std::f64::consts::FRAC_PI_2;
    // A stands in the middle of the crossing at speed 0 with no way to leave it (its desired
    // speed is 0), B approaches on the other lane: B waits, then goes after the timeout.
    let north = straight(0.0, 200.0);
    let east = LaneSegment::straight(DVec2::new(-100.0, 100.0), FRAC_PI_2, 200.0, LaneKind::Road);
    let mut w = world(&[north, east]);
    let parked = IdmParams {
        desired_speed: 0.1,
        max_accel: 0.0,
        ..IdmParams::default()
    };
    w.spawn(LaneId(0), 101.0, 0.0, parked).unwrap();
    let b = w.spawn(LaneId(1), 40.0, 8.0, IdmParams::default()).unwrap();
    for _ in 0..600 {
        w.step(0.1);
    }
    assert!(
        w.agent(b).is_none_or(|a| a.s > 100.0),
        "B never passed the crossing"
    );
}
