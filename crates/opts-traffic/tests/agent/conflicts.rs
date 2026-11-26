//! Crossings, signals and priority: the parts that use only the public API. Moved out of `src/` by ISS-094.

use super::*;

mod lights;
mod priority;

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
