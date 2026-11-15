//! Lane changes to overtake a slow leader: timing, priority of the target lane and the glide.

use super::*;

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
