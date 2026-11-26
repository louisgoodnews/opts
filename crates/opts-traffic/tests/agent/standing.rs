//! Standing vehicles: the clearance rules. Moved out of `src/` by ISS-094.

use super::*;

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

mod clearance;
