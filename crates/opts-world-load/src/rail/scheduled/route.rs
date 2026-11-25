//! Route planning, stop-distance control and retirement for scheduled trains and trams.

use super::*;

mod retire;
mod stop;

pub(in super::super) use retire::{retired, run_of};
pub(in super::super) use stop::control;
#[cfg(test)]
pub(in super::super) use stop::{distance_to, Ahead};

/// Longest way (m) over the tracks between two stops.
const LEG_LIMIT_M: f32 = 25_000.0;
/// A train is removed this long (s) after arriving at its last stop, if the camera is farther
/// than `RETIRE_FAR_M` (m) away.
const RETIRE_AFTER_S: f64 = 8.0;
const RETIRE_FAR_M: f32 = 80.0;

/// A stop object's place on the tracks: lane, arc length, position (session frame).
#[derive(Debug, Clone, Copy)]
pub(in super::super) struct Place {
    pub(in super::super) lane: LaneId,
    pub(in super::super) s: f32,
}

/// The walking coordinate along `states` (lanes with their direction) of arc length `s` on lane
/// number `i`: metres from the start of the first lane in walking direction.
pub(in super::super) fn walk_u(
    net: &PedestrianNet,
    states: &[(LaneId, i8)],
    i: usize,
    s: f32,
) -> f32 {
    let before: f32 = states[..i]
        .iter()
        .map(|(l, _)| net.lane(*l).map_or(0.0, |l| l.length()))
        .sum();
    let len = net.lane(states[i].0).map_or(0.0, |l| l.length());
    before + if states[i].1 > 0 { s } else { len - s }
}

/// The lane index and arc length at walking coordinate `u` along `states`.
pub(in super::super) fn walk_at(
    net: &PedestrianNet,
    states: &[(LaneId, i8)],
    u: f32,
) -> Option<(usize, f32)> {
    let mut left = u.max(0.0);
    for (i, (l, d)) in states.iter().enumerate() {
        let len = net.lane(*l)?.length();
        if left <= len || i + 1 == states.len() {
            let left = left.min(len);
            return Some((i, if *d > 0 { left } else { len - left }));
        }
        left -= len;
    }
    None
}

/// The tracks a run takes through its stops.
pub(in super::super) struct RailRoute {
    /// The lanes in running order, each with the direction it is walked in; the first is the
    /// first stop's.
    pub(in super::super) states: Vec<(LaneId, i8)>,
    /// Index in `states` of the lane of each stop (parallel to the places).
    pub(in super::super) stop_index: Vec<usize>,
    /// Walking coordinate of each stop object's place.
    pub(in super::super) stop_u: Vec<f32>,
}

/// Whether route `a` is better than `b`: more stops reached, then the shorter way.
pub(in super::super) fn better(a: &RailRoute, b: &RailRoute) -> bool {
    match a.stop_u.len().cmp(&b.stop_u.len()) {
        std::cmp::Ordering::Greater => true,
        std::cmp::Ordering::Less => false,
        std::cmp::Ordering::Equal => {
            a.stop_u.last().unwrap_or(&f32::MAX) < b.stop_u.last().unwrap_or(&f32::MAX)
        }
    }
}

/// The way over the track network from `start` (lane, arc length) walking in direction `dir`
/// through the `targets`; the route ends at the last target that can be reached. `None` when the
/// first target cannot be reached.
pub(in super::super) fn build_route(
    net: &PedestrianNet,
    (start, dir): (Place, i8),
    targets: &[Place],
) -> Option<RailRoute> {
    let mut states = vec![(start.lane, dir)];
    let mut stop_index = Vec::new();
    let mut stop_u = Vec::new();
    let mut at_s = start.s;
    for t in targets {
        let (from, d) = *states.last()?;
        if from == t.lane {
            // On the same lane: ahead of the head if the walk goes that way (a few metres of
            // slack for a stop object beside the head).
            if (t.s - at_s) * f32::from(d) < -3.0 {
                break;
            }
        } else {
            match net.shortest_path(from, d, t.lane, LEG_LIMIT_M) {
                Some(p) => states.extend(p),
                None => break,
            }
        }
        let i = states.len() - 1;
        stop_index.push(i);
        stop_u.push(walk_u(net, &states, i, t.s));
        at_s = t.s;
    }
    (!stop_index.is_empty()).then_some(RailRoute {
        states,
        stop_index,
        stop_u,
    })
}
