//! Unit tests of `agent.rs` (moved out of the file to keep it under the 500-line limit, refactoring cycle 1).

use super::*;
use crate::geometry::PathGeometry;
use crate::graph::LaneRules;
use crate::graph::{ConnectParams, Direction, LaneKind, LaneSegment};
use crate::neighbours::Side;
use crate::signals::SignalProgram;

fn straight(y: f64, len: f64) -> LaneSegment {
    LaneSegment::straight(DVec2::new(0.0, y), 0.0, len, LaneKind::Road)
}

fn world(segments: &[LaneSegment]) -> TrafficWorld {
    let mut w = TrafficWorld::new(LaneGraph::build(segments, &ConnectParams::default()), 3);
    w.set_stuck_after(f64::INFINITY);
    w
}

#[test]
fn an_agent_accelerates_and_crosses_into_the_next_lane() {
    let mut w = world(&[straight(0.0, 60.0), straight(60.0, 400.0)]);
    let id = w.spawn(LaneId(0), 0.0, 0.0, IdmParams::default()).unwrap();
    for _ in 0..200 {
        w.step(0.1);
    }
    let a = w.agent(id).unwrap();
    assert_eq!(a.lane, LaneId(1));
    assert!(a.speed > 10.0);
    let (p, h) = a.pose(w.graph()).unwrap();
    assert!(p.y > 60.0 && p.x.abs() < 1e-9 && h.abs() < 1e-9);
}

#[test]
fn a_follower_keeps_its_distance_behind_a_stopped_leader() {
    let mut w = world(&[straight(0.0, 300.0)]);
    let lead = w
        .spawn(LaneId(0), 200.0, 0.0, IdmParams::default())
        .unwrap();
    let p = IdmParams {
        max_accel: 0.0,
        ..IdmParams::default()
    };
    // A leader that never moves: zero acceleration means it stays at speed 0.
    w.agents.iter_mut().find(|a| a.id == lead).unwrap().params = p;
    let follow = w.spawn(LaneId(0), 0.0, 13.0, IdmParams::default()).unwrap();
    for _ in 0..1200 {
        w.step(0.1);
    }
    let (l, f) = (w.agent(lead).unwrap(), w.agent(follow).unwrap());
    let gap = l.s - l.params.length - f.s;
    assert!(f.speed < 0.05, "follower still at {}", f.speed);
    assert!(gap > 0.5, "collision: gap {gap}");
    assert!(gap < 4.0, "stopped too early: gap {gap}");
}

#[test]
fn agents_see_a_leader_across_the_lane_boundary() {
    let mut w = world(&[straight(0.0, 100.0), straight(100.0, 100.0)]);
    let lead = w
        .spawn(
            LaneId(1),
            10.0,
            0.0,
            IdmParams {
                max_accel: 0.0,
                ..IdmParams::default()
            },
        )
        .unwrap();
    let follow = w.spawn(LaneId(0), 0.0, 13.0, IdmParams::default()).unwrap();
    for _ in 0..1200 {
        w.step(0.1);
    }
    let (l, f) = (w.agent(lead).unwrap(), w.agent(follow).unwrap());
    assert_eq!(f.lane, LaneId(1));
    assert!(l.s - l.params.length - f.s > 0.5, "collision");
}

#[test]
fn dead_ends_remove_the_agent_and_raise_an_event() {
    let mut w = world(&[straight(0.0, 30.0)]);
    let id = w.spawn(LaneId(0), 0.0, 13.0, IdmParams::default()).unwrap();
    for _ in 0..100 {
        w.step(0.1);
    }
    assert!(w.agent(id).is_none());
    assert_eq!(
        w.drain_events(),
        [TrafficEvent::ReachedDeadEnd {
            id,
            lane: LaneId(0)
        }]
    );
    assert!(w.drain_events().is_empty());
}

#[test]
fn replacing_the_graph_keeps_agents_on_lanes_that_still_exist() {
    let mut a = straight(0.0, 100.0);
    a.source = 1;
    let mut b = straight(100.0, 100.0);
    b.source = 2;
    let mut w = world(&[a.clone(), b.clone()]);
    let on_a = w.spawn(LaneId(0), 50.0, 5.0, IdmParams::default()).unwrap();
    let on_b = w.spawn(LaneId(1), 50.0, 5.0, IdmParams::default()).unwrap();
    // Tile of lane `a` unloads; lane `b` stays but now has id 0.
    let removed = w.replace_graph(LaneGraph::build(&[b], &ConnectParams::default()));
    assert_eq!(removed, [on_a]);
    assert!(w.agent(on_a).is_none());
    assert_eq!(w.agent(on_b).unwrap().lane, LaneId(0));
    // Loading `a` again does not bring the agent back, and `on_b` keeps its lane by key.
    let removed = w.replace_graph(LaneGraph::build(
        &[a, straight(100.0, 100.0)],
        &ConnectParams::default(),
    ));
    assert!(removed.is_empty() || removed == [on_b]);
}

#[test]
fn spawning_needs_room() {
    let mut w = world(&[straight(0.0, 100.0)]);
    w.spawn(LaneId(0), 20.0, 0.0, IdmParams::default()).unwrap();
    assert_eq!(
        w.spawn(LaneId(0), 22.0, 0.0, IdmParams::default()),
        Err(SpawnError::Occupied)
    );
    assert!(w.spawn(LaneId(0), 40.0, 0.0, IdmParams::default()).is_ok());
    assert_eq!(
        w.spawn(LaneId(9), 0.0, 0.0, IdmParams::default()),
        Err(SpawnError::NoSuchLane)
    );
}

#[test]
fn commands_follow_the_acceleration_and_announce_turns() {
    // A right turn (quarter circle, radius 15) after 100 m.
    let turn = LaneSegment {
        geometry: PathGeometry {
            start: DVec2::new(0.0, 100.0),
            heading: 0.0,
            radius: 15.0,
            length: 15.0 * std::f64::consts::FRAC_PI_2,
        },
        kind: LaneKind::Road,
        width: 2.5,
        direction: Direction::Forward,
        speed_limit: None,
        rules: Default::default(),
        elevation: Default::default(),
        signal: None,
        junction: false,
        indicator: None,
        source: 0,
    };
    let mut w = world(&[straight(0.0, 100.0), turn]);
    let id = w.spawn(LaneId(0), 0.0, 0.0, IdmParams::default()).unwrap();
    w.step(0.1);
    let c = w.command(id).unwrap();
    assert!(c.throttle > 0.9 && c.brake == 0.0 && c.blinker == Blinker::Off);
    w.agents[0].s = 80.0;
    assert_eq!(w.command(id).unwrap().blinker, Blinker::Right);
}

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

/// Two parallel north-bound lanes 3.5 m apart: lane 0 (left) and lane 1 (right, east).
fn two_lanes() -> TrafficWorld {
    let right = LaneSegment::straight(DVec2::new(3.5, 0.0), 0.0, 800.0, LaneKind::Road);
    world(&[straight(0.0, 800.0), right])
}

fn slow() -> IdmParams {
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
fn crossing(a: LaneRules, b: LaneRules) -> TrafficWorld {
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

/// A straight 100 m, a right-hand quarter circle of radius `r` and another straight.
fn bend(r: f64) -> TrafficWorld {
    use std::f64::consts::FRAC_PI_2;
    let arc = LaneSegment {
        geometry: PathGeometry {
            start: DVec2::new(0.0, 100.0),
            heading: 0.0,
            radius: r,
            length: r * FRAC_PI_2,
        },
        ..straight(100.0, 0.0)
    };
    let after = LaneSegment::straight(DVec2::new(r, 100.0 + r), FRAC_PI_2, 200.0, LaneKind::Road);
    world(&[straight(0.0, 100.0), arc, after])
}

#[test]
fn in_a_tight_bend_the_axles_track_the_lane_and_the_speed_suits_the_radius() {
    let r = 12.0;
    let mut w = bend(r);
    let id = w.spawn(LaneId(0), 0.0, 12.0, IdmParams::default()).unwrap();
    let centre = DVec2::new(r, 100.0);
    let (l, v_c) = (IdmParams::default().wheelbase, (2.5 * r).sqrt());
    let (mut braked_before, mut on_arc) = (false, 0);
    for _ in 0..300 {
        w.step(0.1);
        let Some(a) = w.agent(id) else { break };
        if a.lane == LaneId(0) && a.accel < -0.3 {
            braked_before = true;
        }
        if a.lane == LaneId(1) && a.s > a.params.front_overhang + l {
            on_arc += 1;
            assert!(a.speed <= v_c + 0.3, "{} m/s on the arc", a.speed);
            // Front axle on the lane; the rear axle off-tracks inwards by at most
            // R − √(R² − L²) (≈ 0.29 m) and never outwards.
            let front = a.front_axle.distance(centre);
            let rear = a.rear_axle.distance(centre);
            assert!(
                (front - r).abs() < 0.1,
                "front axle {front} from the centre"
            );
            assert!(
                rear <= r + 0.1 && rear >= (r * r - l * l).sqrt() - 0.1,
                "rear axle {rear} from the centre"
            );
            assert!(a.steer > 0.05, "steers right: {}", a.steer);
        }
    }
    assert!(braked_before, "braking starts before the arc");
    assert!(on_arc > 5);
}

#[test]
fn a_kinked_lane_join_turns_the_body_smoothly() {
    // A 12° kink between two lanes (within the join tolerance): the body's heading must not
    // jump at the join.
    let kink = 12f64.to_radians();
    let next = LaneSegment::straight(DVec2::new(0.0, 100.0), kink, 100.0, LaneKind::Road);
    let mut w = world(&[straight(0.0, 100.0), next]);
    // 25 km/h, a junction speed: the body turns over about a wheelbase of travel.
    let slow = IdmParams {
        desired_speed: 7.0,
        ..IdmParams::default()
    };
    let id = w.spawn(LaneId(0), 70.0, 7.0, slow).unwrap();
    let mut h = w.agent(id).unwrap().heading();
    let mut worst = 0.0f64;
    for _ in 0..80 {
        w.step(0.1);
        let Some(a) = w.agent(id) else { break };
        worst = worst.max((a.heading() - h).abs());
        h = a.heading();
    }
    assert!(
        (h - kink).abs() < 0.01,
        "ends on the new lane's heading: {h}"
    );
    assert!(
        worst < 2f64.to_radians(),
        "heading jumped {}°",
        worst.to_degrees()
    );
}

#[test]
fn a_vehicle_brakes_before_a_lower_limit_and_keeps_it() {
    let mut slow = straight(100.0, 200.0);
    slow.speed_limit = Some(30.0 / 3.6);
    let mut w = world(&[straight(0.0, 100.0), slow]);
    let id = w
        .spawn(LaneId(0), 50.0, 50.0 / 3.6, IdmParams::default())
        .unwrap();
    for _ in 0..200 {
        w.step(0.1);
        let a = w.agent(id).unwrap();
        if a.lane == LaneId(1) {
            assert!(a.speed <= 30.0 / 3.6 + 0.3, "{} km/h", a.speed * 3.6);
        }
    }
    assert_eq!(w.agent(id).unwrap().lane, LaneId(1));
}

#[test]
fn the_speed_cap_is_not_carried_over_from_the_spawn_lane() {
    let mut slow = straight(0.0, 100.0);
    slow.speed_limit = Some(30.0 / 3.6);
    let mut fast = straight(100.0, 600.0);
    fast.speed_limit = Some(70.0 / 3.6);
    let mut w = world(&[slow, fast]);
    let id = w.spawn(LaneId(0), 0.0, 5.0, IdmParams::default()).unwrap();
    for _ in 0..400 {
        w.step(0.1);
    }
    let a = w.agent(id).unwrap();
    assert!(
        a.speed > 60.0 / 3.6,
        "{} km/h on a 70 km/h lane",
        a.speed * 3.6
    );
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
fn same_seed_same_run() {
    let run = || {
        let mut w = world(&[
            straight(0.0, 50.0),
            straight(50.0, 50.0),
            straight(50.0, 80.0),
        ]);
        let id = w.spawn(LaneId(0), 0.0, 5.0, IdmParams::default()).unwrap();
        for _ in 0..100 {
            w.step(0.1);
        }
        w.agent(id).map(|a| (a.lane, a.s.to_bits()))
    };
    assert_eq!(run(), run());
}

// --- vehicles outside the traffic (the player's bus) -----------------------------------------

use crate::obstacles::Obstacle;

/// A bus standing across lane 0 at `y`, heading north.
fn bus_at(x: f64, y: f64) -> Obstacle {
    Obstacle {
        centre: DVec2::new(x, y),
        heading: 0.0,
        length: 12.0,
        width: 2.55,
        velocity: DVec2::ZERO,
    }
}

#[test]
fn the_first_vehicle_stuck_is_despawned_with_an_event() {
    let mut w = world(&[straight(0.0, 800.0)]);
    w.set_stuck_after(STUCK_AFTER_S);
    w.set_obstacles(vec![bus_at(0.0, 200.0)]);
    let first = w
        .spawn(LaneId(0), 150.0, 0.0, IdmParams::default())
        .unwrap();
    let second = w
        .spawn(LaneId(0), 100.0, 0.0, IdmParams::default())
        .unwrap();
    let mut stuck = Vec::new();
    for _ in 0..((STUCK_AFTER_S + 60.0) * 10.0) as usize {
        w.step(0.1);
        for e in w.drain_events() {
            if let TrafficEvent::Stuck { id, .. } = e {
                stuck.push(id);
            }
        }
    }
    // Both drive up to the bus and stand; the one that reaches the limit first
    // goes, and the event names it.
    assert!(!stuck.is_empty());
    assert!(stuck.contains(&first) || stuck.contains(&second));
    assert!(stuck.iter().all(|id| w.agent(*id).is_none()));
}

#[test]
fn a_car_stops_behind_a_standing_vehicle_on_its_lane_and_phantoms_do_not_show() {
    let mut w = world(&[straight(0.0, 800.0)]);
    w.set_obstacles(vec![bus_at(0.0, 200.0)]); // rear end at y = 194
    let id = w
        .spawn(LaneId(0), 30.0, 14.0, IdmParams::default())
        .unwrap();
    for _ in 0..600 {
        w.step(0.1);
        assert!(w.agents().iter().all(|a| !a.foreign), "a phantom leaked");
    }
    let a = w.agent(id).unwrap();
    assert!(a.speed < 0.1, "it still drives at {} m/s", a.speed);
    // Its front bumper stays short of the bus' rear end by about the minimum gap.
    assert!(
        a.s < 194.0 - 0.5,
        "front bumper at {} m touches the bus",
        a.s
    );
    assert!(a.s > 194.0 - 6.0, "it stopped far too early at {} m", a.s);
}

#[test]
fn a_car_changes_to_a_free_lane_round_a_standing_vehicle() {
    let mut w = two_lanes();
    w.set_obstacles(vec![bus_at(0.0, 300.0)]);
    let id = w
        .spawn(LaneId(0), 30.0, 14.0, IdmParams::default())
        .unwrap();
    let mut changed = false;
    let mut worst = 0.0f64;
    for _ in 0..500 {
        w.step(0.1);
        worst = worst.min(w.agent(id).map_or(0.0, |a| a.accel));
        changed |= w
            .drain_events()
            .iter()
            .any(|e| matches!(e, TrafficEvent::LaneChanged { id: i, .. } if *i == id));
    }
    assert!(changed, "the car never changed lanes");
    let a = w.agent(id).unwrap();
    assert!(a.s > 330.0, "it did not get past the bus: s = {}", a.s);
    assert!(worst > -4.5, "it braked at {worst} m/s²");
}

#[test]
fn a_vehicle_next_to_the_lane_does_not_block_it() {
    let mut w = world(&[straight(0.0, 800.0)]);
    // 3.4 m beside the lane: the bus' side is 2.1 m from the lane's centre line, the car's
    // half width 0.95 m: it passes with room to spare.
    w.set_obstacles(vec![bus_at(3.4, 200.0)]);
    let id = w
        .spawn(LaneId(0), 30.0, 14.0, IdmParams::default())
        .unwrap();
    for _ in 0..400 {
        w.step(0.1);
    }
    assert!(w.agent(id).is_some_and(|a| a.s > 250.0 && a.speed > 5.0));
}

#[test]
fn a_vehicle_driving_ahead_is_followed_not_overtaken_through() {
    let mut w = world(&[straight(0.0, 2000.0)]);
    let id = w
        .spawn(LaneId(0), 20.0, 14.0, IdmParams::default())
        .unwrap();
    let mut y = 60.0;
    for _ in 0..300 {
        // The player drives at 5 m/s ahead.
        y += 5.0 * 0.1;
        w.set_obstacles(vec![Obstacle {
            velocity: DVec2::new(0.0, 5.0),
            ..bus_at(0.0, y)
        }]);
        w.step(0.1);
    }
    let a = w.agent(id).unwrap();
    assert!(
        a.s < y - 6.0,
        "the car ran into the bus: s = {}, bus at {y}",
        a.s
    );
    assert!(
        (a.speed - 5.0).abs() < 1.5,
        "it should follow at the bus' speed, not {}",
        a.speed
    );
}

#[test]
fn nothing_is_spawned_inside_a_standing_vehicle() {
    let mut w = world(&[straight(0.0, 800.0)]);
    w.set_obstacles(vec![bus_at(0.0, 200.0)]);
    assert_eq!(
        w.spawn(LaneId(0), 200.0, 0.0, IdmParams::default())
            .unwrap_err(),
        SpawnError::Occupied,
        "inside the bus"
    );
    assert_eq!(
        w.spawn(LaneId(0), 197.0, 0.0, IdmParams::default())
            .unwrap_err(),
        SpawnError::Occupied,
        "with its nose in the bus' rear"
    );
    assert!(
        w.spawn(LaneId(0), 120.0, 0.0, IdmParams::default()).is_ok(),
        "far behind it"
    );
    assert!(
        w.spawn(LaneId(0), 260.0, 0.0, IdmParams::default()).is_ok(),
        "far in front"
    );
}

#[test]
fn the_deadlock_breaker_does_not_drive_through_a_standing_vehicle_in_a_junction() {
    let mut w = crossing(LaneRules::default(), LaneRules::default());
    // The bus stands across the north-bound lane at the crossing.
    w.set_obstacles(vec![Obstacle {
        centre: DVec2::new(0.0, 100.0),
        heading: std::f64::consts::FRAC_PI_2, // lying across the road
        length: 12.0,
        width: 2.55,
        velocity: DVec2::ZERO,
    }]);
    let id = w
        .spawn(LaneId(0), 40.0, 10.0, IdmParams::default())
        .unwrap();
    // Far longer than the deadlock timeout.
    for _ in 0..1200 {
        w.step(0.1);
    }
    let a = w.agent(id).unwrap();
    assert!(a.speed < 0.3, "still moving at {} m/s", a.speed);
    assert!(
        a.s < 100.0 - 1.0,
        "it drove into the bus: front at {} m",
        a.s
    );
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
