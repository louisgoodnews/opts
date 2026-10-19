//! Crossings, signals, priority and the deadlock breaker.

use super::*;

/// Smallest distance between the bodies (front bumper to rear) of two agents over a run.
fn min_body_distance(w: &mut TrafficWorld, steps: usize) -> f64 {
    let mut min = f64::INFINITY;
    for _ in 0..steps {
        w.step(0.1);
        let pts: Vec<Vec<DVec2>> = w
            .agents()
            .iter()
            .map(|a| {
                let g = &w.graph().lane(a.lane).unwrap().geometry;
                (0..=4)
                    .map(|k| g.point_at((a.s - a.params.length * f64::from(k) / 4.0).max(0.0)))
                    .collect()
            })
            .collect();
        for i in 0..pts.len() {
            for j in i + 1..pts.len() {
                for p in &pts[i] {
                    for q in &pts[j] {
                        min = min.min(p.distance(*q));
                    }
                }
            }
        }
    }
    min
}

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

/// A lane of 100 m followed by a lane governed by light `(1, 0)`.
fn light_world(program: SignalProgram) -> TrafficWorld {
    let mut gated = straight(100.0, 200.0);
    gated.signal = Some((1, 0));
    let mut w = world(&[straight(0.0, 100.0), gated]);
    w.set_signals(HashMap::from([((1, 0), program)]));
    w.set_clock(0.0);
    w
}

#[test]
fn vehicles_stop_at_a_red_light_and_go_on_green() {
    let mut w = light_world(SignalProgram::new(vec![(0, 20.0), (6, 100.0)], 50.0).unwrap());
    let id = w.spawn(LaneId(0), 0.0, 12.0, IdmParams::default()).unwrap();
    for _ in 0..150 {
        w.step(0.1);
    }
    let a = w.agent(id).unwrap();
    assert_eq!(a.lane, LaneId(0), "ran the red light");
    assert!(
        a.speed < 0.2 && a.s > 90.0 && a.s < 100.0,
        "stopped at s = {}",
        a.s
    );
    for _ in 0..100 {
        w.step(0.1);
    }
    let a = w.agent(id).unwrap();
    assert_eq!(a.lane, LaneId(1));
    assert!(a.speed > 5.0);
}

#[test]
fn a_vehicle_too_close_to_stop_runs_the_yellow() {
    let mut w = light_world(SignalProgram::new(vec![(9, 10.0), (0, 30.0)], 0.0).unwrap());
    let late = w
        .spawn(LaneId(0), 90.0, 14.0, IdmParams::default())
        .unwrap();
    for _ in 0..30 {
        w.step(0.1);
    }
    assert_eq!(
        w.agent(late).unwrap().lane,
        LaneId(1),
        "stopped hard for a yellow"
    );
    assert!(w.agent(late).unwrap().speed > 10.0);
}

/// Two perpendicular roads crossing at (0, 100): lane 0 north-bound, lane 1 west-bound (it comes
/// from the north-bound driver's right), both 200 m long.
pub(super) fn crossing(a: LaneRules, b: LaneRules) -> TrafficWorld {
    use std::f64::consts::FRAC_PI_2;
    let mut north = straight(0.0, 200.0);
    north.rules = a;
    let mut west = LaneSegment::straight(
        DVec2::new(100.0, 100.0),
        3.0 * FRAC_PI_2,
        200.0,
        LaneKind::Road,
    );
    west.rules = b;
    world(&[north, west])
}

/// Slowest speed each agent had while it was within 60 m of the crossing, plus the closest
/// approach of the two bodies.
fn run_crossing(w: &mut TrafficWorld, steps: usize) -> ([f64; 2], f64) {
    let mut slowest = [f64::INFINITY; 2];
    let mut closest = f64::INFINITY;
    for _ in 0..steps {
        w.step(0.1);
        let mut pos = Vec::new();
        for (k, a) in w.agents().iter().enumerate() {
            let near = a.s > 40.0 && a.s < 130.0;
            if near {
                slowest[a.id.0 as usize - 1] = slowest[a.id.0 as usize - 1].min(a.speed);
            }
            let g = &w.graph().lane(a.lane).unwrap().geometry;
            pos.push((k, g.point_at(a.s)));
        }
        if pos.len() == 2 {
            closest = closest.min(pos[0].1.distance(pos[1].1));
        }
    }
    (slowest, closest)
}

#[test]
fn right_before_left_the_vehicle_from_the_right_goes_first() {
    let mut w = crossing(LaneRules::default(), LaneRules::default());
    // Both reach the crossing together; the north-bound one has the other on its right.
    w.spawn(LaneId(0), 60.0, 12.0, IdmParams::default())
        .unwrap();
    w.spawn(LaneId(1), 60.0, 12.0, IdmParams::default())
        .unwrap();
    let (slowest, closest) = run_crossing(&mut w, 150);
    assert!(
        slowest[1] > 10.0,
        "the vehicle from the right slowed to {}",
        slowest[1]
    );
    assert!(
        slowest[0] < 6.0,
        "the vehicle from the left did not give way ({})",
        slowest[0]
    );
    assert!(closest > 1.4, "bodies came within {closest} m");
}

#[test]
fn a_priority_road_goes_first_even_when_the_side_road_arrives_earlier() {
    let high = LaneRules {
        priority: Some(192),
        ..LaneRules::default()
    };
    let low = LaneRules {
        priority: Some(64),
        ..LaneRules::default()
    };
    // The west-bound vehicle is on the left-hand priority... flip: lane 0 is the priority road
    // although lane 1 comes from its right, and lane 1 is 10 m closer to the crossing.
    let mut w = crossing(high, low);
    w.spawn(LaneId(0), 50.0, 12.0, IdmParams::default())
        .unwrap();
    w.spawn(LaneId(1), 60.0, 12.0, IdmParams::default())
        .unwrap();
    let (slowest, closest) = run_crossing(&mut w, 150);
    assert!(
        slowest[0] > 10.0,
        "the priority road was held up (slowest {})",
        slowest[0]
    );
    assert!(
        slowest[1] < 6.0,
        "the side road did not give way ({})",
        slowest[1]
    );
    assert!(closest > 1.4, "bodies came within {closest} m");
}

#[test]
fn vehicles_without_right_of_way_do_not_wait_for_far_away_traffic() {
    let mut w = crossing(LaneRules::default(), LaneRules::default());
    // The vehicle from the right is 150 m away (12 s): the north-bound one just drives on.
    w.spawn(LaneId(0), 60.0, 12.0, IdmParams::default())
        .unwrap();
    w.spawn(LaneId(1), 0.0, 12.0, IdmParams::default()).unwrap();
    let (slowest, _) = run_crossing(&mut w, 80);
    assert!(slowest[0] > 10.0, "slowed to {}", slowest[0]);
}

#[test]
fn a_vehicle_waiting_at_red_does_not_hold_up_crossing_traffic() {
    use std::f64::consts::FRAC_PI_2;
    // East-bound approach lane, then the signalled lane across the north-bound road at x = 0.
    let approach =
        LaneSegment::straight(DVec2::new(-100.0, 100.0), FRAC_PI_2, 97.0, LaneKind::Road);
    let mut gated =
        LaneSegment::straight(DVec2::new(-3.0, 100.0), FRAC_PI_2, 106.0, LaneKind::Road);
    gated.signal = Some((7, 0));
    let mut w = world(&[straight(0.0, 200.0), approach, gated]);
    w.set_signals(HashMap::from([(
        (7, 0),
        SignalProgram::new(vec![(0, 60.0), (6, 60.0)], 0.0).unwrap(),
    )]));
    w.set_clock(0.0);
    w.spawn(LaneId(1), 95.0, 0.0, IdmParams::default()).unwrap();
    let north = w
        .spawn(LaneId(0), 60.0, 12.0, IdmParams::default())
        .unwrap();
    let mut slowest = f64::INFINITY;
    for _ in 0..80 {
        w.step(0.1);
        if let Some(a) = w.agent(north) {
            if a.s < 110.0 {
                slowest = slowest.min(a.speed);
            }
        }
    }
    assert!(slowest > 9.0, "the crossing car slowed to {slowest} m/s");
}

/// Four lanes into one junction, one vehicle on each, all arriving together: right before left
/// is a cycle (everyone yields to someone). The breaker has to let them through one at a time.
fn four_way() -> TrafficWorld {
    use std::f64::consts::{FRAC_PI_2, PI};
    let lane =
        |x: f64, y: f64, h: f64| LaneSegment::straight(DVec2::new(x, y), h, 200.0, LaneKind::Road);
    let mut w = world(&[
        // Right-hand traffic: each direction keeps 1.75 m to the right of the axis.
        lane(1.75, -100.0, 0.0),            // north-bound
        lane(-100.0, -1.75, FRAC_PI_2),     // east-bound
        lane(-1.75, 100.0, PI),             // south-bound
        lane(100.0, 1.75, 3.0 * FRAC_PI_2), // west-bound
    ]);
    // The deadlock breaker is what is under test: the vehicles must not be removed as stuck.
    w.set_stuck_after(f64::INFINITY);
    w
}

#[test]
fn a_four_way_cycle_resolves_one_vehicle_at_a_time_without_touching() {
    let mut w = four_way();
    let ids: Vec<AgentId> = (0..4)
        .map(|l| {
            w.spawn(LaneId(l), 70.0, 12.0, IdmParams::default())
                .unwrap()
        })
        .collect();
    let min = min_body_distance(&mut w, 1500);
    assert!(min > 1.4, "bodies came within {min} m in the junction");
    assert!(
        ids.iter().all(|id| w.agent(*id).is_none()),
        "somebody is still stuck in the junction"
    );
}

#[test]
fn a_released_vehicle_still_waits_for_one_that_is_driving_through_the_zone() {
    // Cross traffic that is *moving* through the zone is respected even by a vehicle the
    // breaker has released: the breaker is for cyclic waits, not for ramming.
    let mut w = four_way();
    // `a` is in the middle of the junction, moving; `b`, released by the breaker, comes up
    // from the side. With the old rule `b` ignored `a` and drove into it.
    let _a = w
        .spawn(LaneId(0), 100.5, 1.0, IdmParams::default())
        .unwrap();
    let b = w
        .spawn(LaneId(1), 86.0, 10.0, IdmParams::default())
        .unwrap();
    w.agents
        .iter_mut()
        .find(|x| x.id == b)
        .unwrap()
        .ignore_conflicts_until = f64::INFINITY;
    let min = min_body_distance(&mut w, 400);
    assert!(
        min > 1.4,
        "the released vehicle touched the one in the zone: {min} m"
    );
}
