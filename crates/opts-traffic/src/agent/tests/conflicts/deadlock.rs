//! Cyclic waits at a four-way junction: the deadlock breaker releases one vehicle at a time.

use super::*;

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
