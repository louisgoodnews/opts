//! Conflict zones between lanes: where two flows cross or merge and must take turns.
//!
//! OMSI states no right of way for most junctions (`docs/src/architecture/ai-traffic.md`, §3.3),
//! so conflicts are found from the geometry alone:
//!
//! * **Crossing** — two lanes pass within [`RADIUS`] of each other at an angle between
//!   [`MIN_ANGLE`] and [`MAX_ANGLE`]. Parallel neighbours, diverging branches (small angle) and
//!   opposing lanes (≈ 180°) are no conflicts. The zone on each lane spans the stretch where they
//!   are that close, widened by [`MARGIN`].
//! * **Merging** — two lanes lead into the same successor; the zone is the last
//!   [`MERGE_ZONE`] metres of each.
//!
//! Conflicts are computed per lane on first use ([`LaneGraph::conflicts`]), so a rebuilt graph
//! stays cheap until agents actually approach a junction.

use std::collections::HashSet;

use glam::DVec2;

use crate::graph::{LaneGraph, LaneId};

/// Two lanes closer than this (m) at some point cross there.
pub const RADIUS: f64 = 1.6;
/// Smallest crossing angle (rad) — below it lanes merge or diverge.
pub const MIN_ANGLE: f64 = 20.0 * std::f64::consts::PI / 180.0;
/// Largest crossing angle (rad) — above it lanes oppose each other.
pub const MAX_ANGLE: f64 = 160.0 * std::f64::consts::PI / 180.0;
/// Zone widening before and after the touching stretch (m).
pub const MARGIN: f64 = 1.0;
/// Length of the merge zone at the end of each merging lane (m).
pub const MERGE_ZONE: f64 = 2.0;
/// Sampling step along lanes (m).
const STEP: f64 = 1.0;
/// Side of the spatial-hash cells (m).
pub(crate) const CELL: f64 = 16.0;

/// How two lanes meet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictKind {
    /// The lanes cross each other.
    Crossing,
    /// The lanes lead into the same successor.
    Merge,
}

/// A zone on one lane that another lane's vehicles also use.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Conflict {
    /// Crossing or merging.
    pub kind: ConflictKind,
    /// The other lane.
    pub other: LaneId,
    /// Start of the zone on this lane, metres from the lane start.
    pub enter: f64,
    /// End of the zone on this lane.
    pub exit: f64,
    /// Start of the zone on the other lane.
    pub other_enter: f64,
    /// End of the zone on the other lane.
    pub other_exit: f64,
}

pub(crate) fn cell(p: DVec2) -> (i64, i64) {
    ((p.x / CELL).floor() as i64, (p.y / CELL).floor() as i64)
}

fn angle_between(a: f64, b: f64) -> f64 {
    let d = (a - b).rem_euclid(std::f64::consts::TAU);
    d.min(std::f64::consts::TAU - d)
}

/// `(s, point, heading)` every [`STEP`] metres and at the end.
fn samples(g: &crate::geometry::PathGeometry) -> Vec<(f64, DVec2, f64)> {
    let n = (g.length / STEP).ceil().max(1.0) as usize;
    (0..=n)
        .map(|i| {
            let s = (i as f64 * STEP).min(g.length);
            (s, g.point_at(s), g.heading_at(s))
        })
        .collect()
}

impl LaneGraph {
    /// The conflict zones of lane `id` (empty for unknown lanes), computed on first use.
    #[must_use]
    pub fn conflicts(&self, id: LaneId) -> &[Conflict] {
        match self.conflict_cache.get(id.0 as usize) {
            Some(cache) => cache.get_or_init(|| self.compute_conflicts(id)),
            None => &[],
        }
    }

    fn compute_conflicts(&self, id: LaneId) -> Vec<Conflict> {
        let Some(lane) = self.lane(id) else {
            return Vec::new();
        };
        let mine = samples(&lane.geometry);
        let mut candidates: HashSet<u32> = HashSet::new();
        for (_, p, _) in &mine {
            let (cx, cy) = cell(*p);
            for dx in -1..=1 {
                for dy in -1..=1 {
                    if let Some(v) = self.grid.get(&(cx + dx, cy + dy)) {
                        candidates.extend(v.iter().copied());
                    }
                }
            }
        }
        candidates.remove(&id.0);
        let mut ordered: Vec<u32> = candidates.into_iter().collect();
        ordered.sort_unstable();
        let mut out: Vec<Conflict> = Vec::new();
        for m in ordered {
            let other = LaneId(m);
            let Some(ol) = self.lane(other) else { continue };
            if ol.kind != lane.kind {
                continue;
            }
            let theirs = samples(&ol.geometry);
            let (mut e, mut x) = (f64::INFINITY, f64::NEG_INFINITY);
            let (mut oe, mut ox) = (f64::INFINITY, f64::NEG_INFINITY);
            for (s, p, h) in &mine {
                for (os, q, oh) in &theirs {
                    if p.distance(*q) >= RADIUS {
                        continue;
                    }
                    let a = angle_between(*h, *oh);
                    if (MIN_ANGLE..=MAX_ANGLE).contains(&a) {
                        e = e.min(*s);
                        x = x.max(*s);
                        oe = oe.min(*os);
                        ox = ox.max(*os);
                    }
                }
            }
            if e <= x {
                out.push(Conflict {
                    kind: ConflictKind::Crossing,
                    other,
                    enter: (e - MARGIN).max(0.0),
                    exit: (x + MARGIN).min(lane.length()),
                    other_enter: (oe - MARGIN).max(0.0),
                    other_exit: (ox + MARGIN).min(ol.length()),
                });
            }
        }
        // Merges: lanes that lead into one of my successors.
        for n in self.successors(id) {
            for &m in self.predecessors(*n) {
                if m == id || out.iter().any(|c| c.other == m) {
                    continue;
                }
                let Some(ol) = self.lane(m) else { continue };
                if ol.kind != lane.kind {
                    continue;
                }
                out.push(Conflict {
                    kind: ConflictKind::Merge,
                    other: m,
                    enter: (lane.length() - MERGE_ZONE).max(0.0),
                    exit: lane.length(),
                    other_enter: (ol.length() - MERGE_ZONE).max(0.0),
                    other_exit: ol.length(),
                });
            }
        }
        out
    }

    /// Indexes lane samples in the spatial hash used by [`Self::conflicts`].
    pub(crate) fn index_lanes(
        lanes: &[crate::graph::Lane],
    ) -> std::collections::HashMap<(i64, i64), Vec<u32>> {
        let mut grid: std::collections::HashMap<(i64, i64), Vec<u32>> =
            std::collections::HashMap::new();
        for (i, l) in lanes.iter().enumerate() {
            let g = &l.geometry;
            let n = (g.length / 4.0).ceil().max(1.0) as usize;
            let mut last = None;
            for k in 0..=n {
                let s = (k as f64 * 4.0).min(g.length);
                let c = cell(g.point_at(s));
                if last != Some(c) {
                    let v = grid.entry(c).or_default();
                    if v.last() != Some(&(i as u32)) {
                        v.push(i as u32);
                    }
                    last = Some(c);
                }
            }
        }
        grid
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{ConnectParams, Direction, LaneKind, LaneSegment};
    use std::f64::consts::FRAC_PI_2;

    fn road(x: f64, y: f64, heading: f64, len: f64) -> LaneSegment {
        LaneSegment::straight(DVec2::new(x, y), heading, len, LaneKind::Road)
    }

    fn graph(segs: &[LaneSegment]) -> LaneGraph {
        LaneGraph::build(segs, &ConnectParams::default())
    }

    #[test]
    fn perpendicular_lanes_conflict_where_they_cross() {
        // North-bound lane at x = 0 and east-bound lane at y = 50 cross at (0, 50).
        let g = graph(&[
            road(0.0, 0.0, 0.0, 100.0),
            road(-50.0, 50.0, FRAC_PI_2, 100.0),
        ]);
        let c = g.conflicts(LaneId(0));
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].other, LaneId(1));
        assert!(
            c[0].enter < 50.0 && c[0].exit > 50.0 && c[0].exit - c[0].enter < 8.0,
            "{c:?}"
        );
        let d = g.conflicts(LaneId(1));
        assert!(d[0].enter < 50.0 && d[0].exit > 50.0);
        assert_eq!(d[0].other, LaneId(0));
    }

    #[test]
    fn parallel_opposing_and_diverging_lanes_do_not_conflict() {
        let mut both = road(0.0, 0.0, 0.0, 100.0);
        both.direction = Direction::Both;
        // A neighbour 3 m to the side and a lane that leaves at 10°.
        let g = graph(&[
            both,
            road(3.0, 0.0, 0.0, 100.0),
            road(0.0, 0.0, 0.17, 100.0),
        ]);
        for id in g.lane_ids() {
            assert!(
                g.conflicts(id).iter().all(|c| c.other == id),
                "lane {id:?}: {:?}",
                g.conflicts(id)
            );
        }
    }

    #[test]
    fn lanes_into_the_same_successor_merge() {
        let g = graph(&[
            road(0.0, 0.0, 0.0, 50.0),
            road(0.0, 50.0, 0.0, 50.0),
            // A second feeder from the west, ending at (0, 50) heading north.
            road(-0.3, 20.0, 0.0, 30.3),
        ]);
        let c = g.conflicts(LaneId(0));
        assert!(
            c.iter().any(|c| c.other == LaneId(2) && c.exit > 49.9),
            "{c:?}"
        );
    }

    #[test]
    fn unknown_lanes_have_no_conflicts() {
        assert!(graph(&[]).conflicts(LaneId(3)).is_empty());
    }
}
