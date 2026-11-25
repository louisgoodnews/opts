//! Placing a run's vehicle on the lanes: distances along a route, legs between stops, walking back
//! from a stop and the speed of a leg.

use opts_traffic::{LaneGraph, LaneId, LaneKind};

/// Where the lane distance `d` along `route` (starting `s0` into its first lane) is: the lane
/// index in the route and the position on it.
pub(super) fn place_along(lengths: &[f64], s0: f64, d: f64) -> Option<(usize, f64)> {
    let mut left = d + s0;
    for (i, len) in lengths.iter().enumerate() {
        if left <= *len || i + 1 == lengths.len() {
            return Some((i, left.clamp(0.0, *len)));
        }
        left -= len;
    }
    None
}

/// The lanes from stop place `a` to stop place `b` (both included) and their lengths.
pub(super) fn leg(
    graph: &LaneGraph,
    a: (LaneId, f64),
    b: (LaneId, f64),
) -> Option<(Vec<LaneId>, Vec<f64>)> {
    let lanes = if a.0 == b.0 && b.1 >= a.1 {
        vec![a.0]
    } else {
        graph.shortest_route(a.0, b.0)?
    };
    let lengths = lanes
        .iter()
        .map(|l| graph.lane(*l).map_or(0.0, |l| l.length()))
        .collect();
    Some((lanes, lengths))
}

/// The lane and arc length `d` metres back from `s` on `lane`, going back through the first
/// road predecessor each time.
pub(super) fn walk_back(graph: &LaneGraph, lane: LaneId, s: f64, d: f64) -> Option<(LaneId, f64)> {
    if d <= s {
        return Some((lane, s - d));
    }
    let (mut lane, mut left) = (lane, d - s);
    for _ in 0..64 {
        let next = graph.predecessors(lane).iter().copied().find(|p| {
            graph
                .lane(*p)
                .is_some_and(|l| l.kind == LaneKind::Road && l.length() > 4.0)
        })?;
        let len = graph.lane(next)?.length();
        if left <= len {
            return Some((next, len - left));
        }
        left -= len;
        lane = next;
    }
    None
}

/// A plausible cruising speed (m/s) for a leg that takes `secs`: the vehicle will adapt.
pub(super) fn graph_leg_speed(secs: f64) -> f64 {
    if secs < 30.0 {
        4.0
    } else {
        9.0
    }
}
