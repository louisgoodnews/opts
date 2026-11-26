//! Unit tests of `train.rs`.

use super::*;
use opts_sim_pedestrian::{FootLane, Travel};
#[path = "train_tests/motion.rs"]
mod motion;
#[path = "train_tests/points.rs"]
mod points;

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
