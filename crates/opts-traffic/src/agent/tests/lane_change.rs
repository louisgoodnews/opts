//! Lane changes and strategic changes.

use super::*;

/// Two parallel north-bound lanes 3.5 m apart: lane 0 (left) and lane 1 (right, east).
pub(super) fn two_lanes() -> TrafficWorld {
    let right = LaneSegment::straight(DVec2::new(3.5, 0.0), 0.0, 800.0, LaneKind::Road);
    world(&[straight(0.0, 800.0), right])
}

pub(super) fn slow() -> IdmParams {
    IdmParams {
        desired_speed: 3.0,
        ..IdmParams::default()
    }
}

#[test]
fn a_car_behind_a_slow_one_changes_lanes_without_hard_braking() {
    let mut w = two_lanes();
    w.spawn(LaneId(0), 110.0, 3.0, slow()).unwrap();
    let fast = w
        .spawn(LaneId(0), 30.0, 14.0, IdmParams::default())
        .unwrap();
    let (mut worst, mut changed) = (0.0f64, false);
    for _ in 0..400 {
        w.step(0.1);
        worst = worst.min(w.agents().iter().map(|a| a.accel).fold(0.0, f64::min));
        changed |= w
            .drain_events()
            .iter()
            .any(|e| matches!(e, TrafficEvent::LaneChanged { id, .. } if *id == fast));
    }
    assert!(changed, "the fast car never changed lanes");
    assert!(worst > -2.2, "someone braked at {worst} m/s²");
    // The slow car did not hold it up for the whole run.
    assert!(w.agent(fast).is_none_or(|a| a.speed > 10.0));
}

#[test]
fn the_target_lane_has_priority_over_a_lane_change() {
    let mut w = two_lanes();
    // A slow leader makes the changer want to leave; a fast car closes in from behind in the
    // target lane, so it has to wait until that car has passed.
    w.spawn(LaneId(0), 124.0, 3.0, slow()).unwrap();
    let changer = w
        .spawn(LaneId(0), 100.0, 9.0, IdmParams::default())
        .unwrap();
    let fast = w
        .spawn(
            LaneId(1),
            55.0,
            20.0,
            IdmParams {
                desired_speed: 20.0,
                // 20 m/s needs a factor on the 50 km/h open-road limit (C3).
                speed_factor: 1.5,
                ..IdmParams::default()
            },
        )
        .unwrap();
    let mut worst_fast = 0.0f64;
    for _ in 0..250 {
        w.step(0.1);
        let (c, f) = (w.agent(changer), w.agent(fast));
        if let (Some(c), Some(f)) = (c, f) {
            worst_fast = worst_fast.min(f.accel);
            // While the fast car is still behind (or beside) the changer, no move into its lane.
            if f.s < c.s + 2.0 {
                assert_eq!(
                    c.lane,
                    LaneId(0),
                    "cut in front of a fast car at t = {}",
                    w.time
                );
            }
        }
    }
    assert!(
        worst_fast > -1.5,
        "the fast car braked at {worst_fast} m/s²"
    );
}

#[test]
fn a_free_right_lane_is_taken_and_the_right_lane_is_kept() {
    let mut w = two_lanes();
    let left = w
        .spawn(LaneId(0), 50.0, 12.0, IdmParams::default())
        .unwrap();
    let right = w
        .spawn(LaneId(1), 300.0, 12.0, IdmParams::default())
        .unwrap();
    let mut events = Vec::new();
    for _ in 0..300 {
        w.step(0.1);
        events.extend(w.drain_events());
    }
    assert!(events.contains(&TrafficEvent::LaneChanged {
        id: left,
        side: Side::Right
    }));
    assert!(!events
        .iter()
        .any(|e| matches!(e, TrafficEvent::LaneChanged { id, .. } if *id == right)));
    assert_eq!(w.agent(left).unwrap().lane, LaneId(1));
}

#[test]
fn a_lane_change_is_continuous_to_the_eye_and_glides_to_the_centre() {
    let mut w = two_lanes();
    w.spawn(LaneId(0), 110.0, 3.0, slow()).unwrap();
    let id = w
        .spawn(LaneId(0), 30.0, 14.0, IdmParams::default())
        .unwrap();
    let mut glided = false;
    for _ in 0..400 {
        let before = w.agent(id).and_then(|a| a.pose(w.graph()));
        w.step(0.1);
        let changed = w
            .drain_events()
            .iter()
            .any(|e| matches!(e, TrafficEvent::LaneChanged { id: who, .. } if *who == id));
        let Some(a) = w.agent(id).cloned() else { break };
        if changed {
            let (b, after) = (before.unwrap().0, a.pose(w.graph()).unwrap().0);
            // Travel plus a little: the car must not jump a lane width sideways.
            assert!(
                b.distance(after) < 14.0 * 0.1 + 0.6,
                "jumped {} m",
                b.distance(after)
            );
            assert!(a.changing_lane() && a.lateral.abs() > 3.0);
            glided = true;
            for _ in 0..32 {
                w.step(0.1);
            }
            let a = w.agent(id).unwrap();
            assert!(
                !a.changing_lane() && a.lateral == 0.0,
                "still offset by {}",
                a.lateral
            );
            break;
        }
    }
    assert!(glided);
}

#[test]
fn no_lane_change_is_started_towards_a_prohibited_overtaking_side() {
    // The right lane is the free one; the changer sits on it and the left lane forbids
    // overtaking, so it must not move left past the slow leader.
    let mut left = straight(0.0, 800.0);
    left.rules.no_overtaking = true;
    let right = LaneSegment::straight(DVec2::new(3.5, 0.0), 0.0, 800.0, LaneKind::Road);
    let mut w = world(&[left, right]);
    w.spawn(LaneId(1), 110.0, 3.0, slow()).unwrap();
    w.spawn(LaneId(1), 30.0, 14.0, IdmParams::default())
        .unwrap();
    for _ in 0..300 {
        w.step(0.1);
    }
    assert!(w.drain_events().iter().all(|e| !matches!(
        e,
        TrafficEvent::LaneChanged {
            side: Side::Left,
            ..
        }
    )));
}

#[test]
fn a_car_stays_out_of_a_trucks_lane() {
    // Whitelist reading (2026-10-19): a `trucks` lane is for trucks only.
    let mut right = LaneSegment::straight(DVec2::new(3.5, 0.0), 0.0, 800.0, LaneKind::Road);
    right.rules.trucks = true;
    let mut w = world(&[straight(0.0, 800.0), right]);
    let car = w
        .spawn(LaneId(0), 50.0, 12.0, IdmParams::default())
        .unwrap();
    let mut events = Vec::new();
    for _ in 0..300 {
        w.step(0.1);
        events.extend(w.drain_events());
    }
    assert!(!events.contains(&TrafficEvent::LaneChanged {
        id: car,
        side: Side::Right
    }));
    assert_eq!(w.agent(car).unwrap().lane, LaneId(0));
}

#[test]
fn a_lane_change_is_possible_after_a_passed_crossing() {
    use std::f64::consts::FRAC_PI_2;
    // Both lanes are crossed at y = 50; the car is 40 m past it behind a slow one.
    let right = LaneSegment::straight(DVec2::new(3.5, 0.0), 0.0, 800.0, LaneKind::Road);
    let cross = LaneSegment::straight(DVec2::new(-50.0, 50.0), FRAC_PI_2, 100.0, LaneKind::Road);
    let mut w = world(&[straight(0.0, 800.0), right, cross]);
    w.spawn(LaneId(1), 170.0, 3.0, slow()).unwrap();
    let fast = w
        .spawn(LaneId(1), 90.0 + 4.4, 12.0, IdmParams::default())
        .unwrap();
    assert!(!w.graph().conflicts(LaneId(1)).is_empty());
    let mut changed = false;
    for _ in 0..150 {
        w.step(0.1);
        changed |= w
            .drain_events()
            .iter()
            .any(|e| matches!(e, TrafficEvent::LaneChanged { id, .. } if *id == fast));
    }
    assert!(changed, "the passed crossing still blocked the change");
}

#[test]
fn a_strategic_change_is_made_before_the_lane_ends() {
    use std::f64::consts::FRAC_PI_2;
    // The left lane ends (no successor); the right one continues into a right turn. The
    // route goes through the neighbour, so the car must move over in time.
    let right = LaneSegment::straight(DVec2::new(3.5, 0.0), 0.0, 200.0, LaneKind::Road);
    let turn = LaneSegment {
        geometry: PathGeometry {
            start: DVec2::new(3.5, 200.0),
            heading: 0.0,
            radius: 15.0,
            length: 15.0 * FRAC_PI_2,
        },
        ..straight(0.0, 0.0)
    };
    let exit = LaneSegment::straight(DVec2::new(18.5, 215.0), FRAC_PI_2, 300.0, LaneKind::Road);
    let mut w = world(&[straight(0.0, 200.0), right, turn, exit]);
    let id = w
        .spawn(LaneId(0), 60.0, 10.0, IdmParams::default())
        .unwrap();
    let s = w
        .agent(id)
        .unwrap()
        .strategic
        .expect("planned through the neighbour");
    assert_eq!((s.from, s.via, s.side), (LaneId(0), LaneId(1), Side::Right));
    let mut events = Vec::new();
    for _ in 0..300 {
        w.step(0.1);
        events.extend(w.drain_events());
    }
    assert!(events.contains(&TrafficEvent::LaneChanged {
        id,
        side: Side::Right
    }));
    assert!(!events
        .iter()
        .any(|e| matches!(e, TrafficEvent::ReachedDeadEnd { .. })));
    assert_eq!(w.agent(id).unwrap().lane, LaneId(3), "took the turn");
}

#[test]
fn the_indicator_of_a_crossing_path_wins_over_the_turn_heuristic() {
    // A straight crossing path that says "left" (parameter 12 = 2), and a right-hand bend
    // that says "none" (0): the path's word counts, before and on the path.
    let mut left = straight(100.0, 30.0);
    left.indicator = Some(Blinker::Left);
    let mut w = world(&[straight(0.0, 100.0), left]);
    let id = w.spawn(LaneId(0), 80.0, 5.0, IdmParams::default()).unwrap();
    assert_eq!(w.command(id).unwrap().blinker, Blinker::Left);
    w.agents[0].s = 40.0;
    assert_eq!(
        w.command(id).unwrap().blinker,
        Blinker::Off,
        "too far ahead"
    );
    w.agents[0].lane = LaneId(1);
    w.agents[0].s = 10.0;
    assert_eq!(
        w.command(id).unwrap().blinker,
        Blinker::Left,
        "kept on the path"
    );
    let quiet = LaneSegment {
        geometry: PathGeometry {
            start: DVec2::new(0.0, 100.0),
            heading: 0.0,
            radius: 15.0,
            length: 15.0 * std::f64::consts::FRAC_PI_2,
        },
        indicator: Some(Blinker::Off),
        ..straight(0.0, 0.0)
    };
    let mut w = world(&[straight(0.0, 100.0), quiet]);
    let id = w.spawn(LaneId(0), 80.0, 5.0, IdmParams::default()).unwrap();
    assert_eq!(w.command(id).unwrap().blinker, Blinker::Off);
}

#[test]
fn a_fork_is_taken_on_the_lane_that_is_not_closed_to_the_cars_group() {
    // Lane 0 ends in a fork: lane 1 is closed to density group 0 (a bus lane), lane 2 is open.
    let mut closed = LaneSegment::straight(DVec2::new(0.0, 100.0), 0.1, 200.0, LaneKind::Road);
    closed.rules.density.set(0, 0.0);
    let open = LaneSegment::straight(DVec2::new(0.0, 100.0), -0.1, 200.0, LaneKind::Road);
    let mut w = world(&[straight(0.0, 100.0), closed, open]);
    assert_eq!(
        w.graph().successors(LaneId(0)).len(),
        2,
        "the fork is connected"
    );
    let cars: Vec<_> = (0..8)
        .map(|i| {
            w.spawn(
                LaneId(0),
                5.0 + f64::from(i) * 11.0,
                10.0,
                IdmParams::default(),
            )
            .unwrap()
        })
        .collect();
    for _ in 0..200 {
        w.step(0.1);
    }
    let mut on_open = 0;
    for id in cars {
        if let Some(a) = w.agent(id) {
            assert_ne!(a.lane, LaneId(1), "a car took the lane closed to its group");
            on_open += usize::from(a.lane == LaneId(2));
        }
    }
    assert!(on_open >= 4, "only {on_open} cars reached the open lane");
}
