//! Building the lane graph: direction expansion, then joining the ends to the starts of the same kind.

use std::collections::HashMap;
use std::sync::OnceLock;

use glam::DVec2;

use crate::geometry::PathGeometry;

use super::*;

pub(super) fn angle_diff(a: f64, b: f64) -> f64 {
    let d = (a - b).rem_euclid(std::f64::consts::TAU);
    d.min(std::f64::consts::TAU - d)
}

impl LaneGraph {
    /// Builds the graph: expands directions, then joins ends to starts of the same kind.
    #[must_use]
    pub fn build(segments: &[LaneSegment], params: &ConnectParams) -> Self {
        let mut lanes = Vec::new();
        let mut per_source: HashMap<u64, u32> = HashMap::new();
        for s in segments {
            let mut mk = |geometry: PathGeometry, elevation: Elevation, reversed: bool| {
                let n = per_source.entry(s.source).or_default();
                let key = (s.source, *n);
                *n += 1;
                Lane {
                    geometry,
                    kind: s.kind,
                    width: s.width,
                    speed_limit: s.speed_limit,
                    rules: s.rules,
                    elevation,
                    signal: s.signal,
                    junction: s.junction,
                    // Driven backwards, a turn indicator would point the wrong way.
                    indicator: if reversed { None } else { s.indicator },
                    source: s.source,
                    key,
                }
            };
            let back = s.elevation.reversed(s.geometry.length);
            match s.direction {
                Direction::Forward => lanes.push(mk(s.geometry, s.elevation, false)),
                Direction::Reverse => lanes.push(mk(s.geometry.reversed(), back, true)),
                Direction::Both => {
                    lanes.push(mk(s.geometry, s.elevation, false));
                    lanes.push(mk(s.geometry.reversed(), back, true));
                }
            }
        }
        // Spatial hash of lane starts.
        let cell = params.position_tolerance.max(0.01) * 2.0;
        let key = |p: DVec2| ((p.x / cell).floor() as i64, (p.y / cell).floor() as i64);
        let mut starts: HashMap<(i64, i64), Vec<u32>> = HashMap::new();
        for (i, l) in lanes.iter().enumerate() {
            starts
                .entry(key(l.geometry.start))
                .or_default()
                .push(i as u32);
        }
        let mut successors = vec![Vec::new(); lanes.len()];
        for (i, a) in lanes.iter().enumerate() {
            let (end, heading) = (a.geometry.end(), a.geometry.end_heading());
            let (cx, cy) = key(end);
            for dx in -1..=1 {
                for dy in -1..=1 {
                    for &j in starts.get(&(cx + dx, cy + dy)).into_iter().flatten() {
                        let b = &lanes[j as usize];
                        // A lane never continues into its own reverse twin (a U-turn on the spot).
                        if j as usize == i
                            || b.kind != a.kind
                            || b.geometry.start.distance(end) > params.position_tolerance
                            || angle_diff(b.geometry.heading, heading) > params.heading_tolerance
                        {
                            continue;
                        }
                        successors[i].push(LaneId(j));
                    }
                }
            }
            successors[i].sort_unstable();
        }
        let mut predecessors = vec![Vec::new(); lanes.len()];
        for (i, succ) in successors.iter().enumerate() {
            for s in succ {
                predecessors[s.0 as usize].push(LaneId(i as u32));
            }
        }
        let grid = Self::index_lanes(&lanes);
        let conflict_cache = (0..lanes.len()).map(|_| OnceLock::new()).collect();
        let neighbour_cache = (0..lanes.len()).map(|_| OnceLock::new()).collect();
        Self {
            lanes,
            successors,
            predecessors,
            grid,
            conflict_cache,
            neighbour_cache,
        }
    }
}
