//! The seeded situations: two parallel north-bound lanes and a slow leader with a changer behind.

use glam::DVec2;
use opts_traffic::{
    AgentId, ConnectParams, IdmParams, LaneGraph, LaneId, LaneKind, LaneSegment, Rng, TrafficWorld,
};

/// Desired speed of the changer (m/s).
pub const CHANGER_DESIRED: f64 = 13.0;

/// Lane 0 (left) and lane 1 (right), 3.5 m apart, both north-bound, 800 m long.
fn two_lanes(seed: u64) -> TrafficWorld {
    let left = LaneSegment::straight(DVec2::new(0.0, 0.0), 0.0, 800.0, LaneKind::Road);
    let right = LaneSegment::straight(DVec2::new(3.5, 0.0), 0.0, 800.0, LaneKind::Road);
    let graph = LaneGraph::build(&[left, right], &ConnectParams::default());
    let mut w = TrafficWorld::new(graph, seed);
    w.set_stuck_after(f64::INFINITY);
    w
}

/// One seeded situation: the slow leader and the changer in lane 0, and maybe a faster car
/// behind the changer's target spot in lane 1. Returns the world and the changer.
pub fn situation(seed: u64) -> (TrafficWorld, AgentId) {
    let mut rng = Rng::new(seed);
    let mut w = two_lanes(seed);
    let leader = IdmParams {
        desired_speed: 3.0,
        ..IdmParams::default()
    };
    w.spawn(LaneId(0), 110.0 + 20.0 * rng.next_f64(), 3.0, leader)
        .expect("the leader fits");
    let changer_params = IdmParams {
        desired_speed: CHANGER_DESIRED,
        ..IdmParams::default()
    };
    let speed = 9.0 + 4.0 * rng.next_f64();
    let s = 60.0 + 30.0 * rng.next_f64();
    let changer = w
        .spawn(LaneId(0), s, speed, changer_params)
        .expect("the changer fits");
    if rng.next_f64() < 0.5 {
        let fast = IdmParams {
            desired_speed: 16.0,
            ..IdmParams::default()
        };
        // Behind the target spot: a follower the changer has to wait for. A spawn that does not
        // fit is simply not made (the situation then has a free target lane).
        let _ = w.spawn(LaneId(1), 20.0 + 40.0 * rng.next_f64(), 14.0, fast);
    }
    (w, changer)
}
