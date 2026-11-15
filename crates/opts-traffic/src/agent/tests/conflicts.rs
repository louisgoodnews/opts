//! Crossings, signals, priority and the deadlock breaker, split by concern.

use super::*;

mod deadlock;
mod junction;
mod lights;
mod priority;

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
