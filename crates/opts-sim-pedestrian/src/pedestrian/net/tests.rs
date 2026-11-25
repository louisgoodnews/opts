//! Unit tests for `super` (`net.rs`): walking and spawning in `tests/walk.rs`, lateral pose in
//! `tests/pose.rs`; the lane fixtures are shared here.

use super::*;

mod pose;
mod walk;

fn lane(a: Vec3, b: Vec3, travel: Travel) -> FootLane {
    FootLane::new(vec![a, b], travel, 1.0).unwrap()
}

fn net() -> PedestrianNet {
    let mut n = PedestrianNet::default();
    // Two bidirectional lanes meeting at (0, 10), a one-way dead end after (0, 20).
    n.insert_tile(
        (0, 0),
        vec![
            lane(Vec3::ZERO, Vec3::new(0.0, 10.0, 0.0), Travel::Both),
            lane(
                Vec3::new(0.0, 10.0, 0.2),
                Vec3::new(0.0, 20.0, 0.2),
                Travel::Both,
            ),
            lane(
                Vec3::new(0.0, 20.0, 0.2),
                Vec3::new(0.0, 30.0, 0.2),
                Travel::Forward,
            ),
        ],
    );
    n
}

/// A 100 m lane running north, `width` metres wide, walkable both ways.
fn north_lane(width: f32) -> PedestrianNet {
    let mut n = PedestrianNet::default();
    let lane = FootLane::new(
        vec![Vec3::ZERO, Vec3::new(0.0, 100.0, 0.0)],
        Travel::Both,
        width,
    )
    .unwrap();
    n.insert_tile((0, 0), vec![lane]);
    n
}

fn walker_on(lane: LaneId, dir: i8, lateral: f32) -> Walker {
    Walker {
        lane,
        s: 50.0,
        dir,
        speed: 1.4,
        phase: 0.0,
        lateral,
    }
}
