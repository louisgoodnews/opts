//! The per-step checks of the trailer tests: passengers stay on the paths and do not jump.

use super::super::*;
use super::fixtures::*;
use glam::Vec3;

pub(super) fn distance_to_segment(p: Vec3, a: Vec3, b: Vec3) -> f32 {
    let ab = b - a;
    let t = if ab.length_squared() > 0.0 {
        ((p - a).dot(ab) / ab.length_squared()).clamp(0.0, 1.0)
    } else {
        0.0
    };
    p.distance(a + ab * t)
}

/// Whether a passenger in `unit` at `p` stands on a link (or a bridge) of the paths, or on the
/// short way between a place and the pathpoint it is reached from.
pub(super) fn on_the_paths(s: &PassengerSim, unit: usize, p: Vec3) -> bool {
    let g = s.graph();
    let reach = g.reachable(&s.cabin.entries);
    let on_link = g
        .links
        .iter()
        .chain(&g.bridges)
        .filter(|&&(a, _)| g.unit(a) == unit)
        .any(|&(a, b)| distance_to_segment(p, g.points[a], g.points[b]) < 0.02);
    let on_stub = s
        .cabin
        .places
        .iter()
        .filter(|pl| pl.unit == unit)
        .any(|pl| {
            g.nearest_reachable(pl.position, unit, &reach)
                .is_some_and(|n| distance_to_segment(p, pl.position, g.points[n]) < 0.02)
        });
    on_link || on_stub
}

/// Runs `seconds` with `doors`, checking that nobody leaves the paths or jumps; returns the exits
/// people got off through.
pub(super) fn run_checked(s: &mut PassengerSim, seconds: f32, doors: &Doors) -> Vec<usize> {
    let mut out = Vec::new();
    let mut last: Vec<(u32, usize, Vec3)> = Vec::new();
    for _ in 0..(seconds / DT) as usize {
        s.step(DT, doors);
        for p in s.passengers() {
            if matches!(p.phase, Phase::WaitingOutside { .. }) {
                continue;
            }
            assert!(
                on_the_paths(s, p.unit, p.position),
                "passenger {} left the paths at {:?} (unit {})",
                p.id,
                p.position,
                p.unit
            );
            if let Some(&(_, u, q)) = last.iter().find(|l| l.0 == p.id) {
                // A step at walking speed; only sitting down puts the person onto the seat.
                if u == p.unit && !matches!(p.phase, Phase::Settled { .. }) {
                    assert!(
                        q.distance(p.position) <= WALK_SPEED_M_S * DT + 1e-3,
                        "passenger {} jumped from {q:?} to {:?}",
                        p.id,
                        p.position
                    );
                }
            }
        }
        last = s
            .passengers()
            .iter()
            .map(|p| (p.id, p.unit, p.position))
            .collect();
        for e in s.take_events() {
            if let Event::Alighted { exit, .. } = e {
                out.push(exit);
            }
        }
    }
    out
}
