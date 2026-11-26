//! Parallel lanes in the same direction: the lanes a vehicle may change to.
//!
//! The data has no notion of "carriageway" or "lane to the left"; two lanes are neighbours when
//! their centre lines run side by side — same direction (within 12°), 2.2 to 5.5 m apart sideways —
//! over at least half of the lane. Roads are built from many short splines whose paths share the
//! spline, so a neighbour usually covers the whole lane.
//!
//! Computed per lane on first use, like conflict zones.

use glam::DVec2;

use crate::conflict::cell;
use crate::graph::{LaneGraph, LaneId};

/// Smallest sideways distance (m) between neighbouring lane centres.
pub const MIN_SEPARATION: f64 = 2.2;
/// Largest sideways distance (m).
pub const MAX_SEPARATION: f64 = 5.5;
/// Largest heading difference (rad) between neighbours.
pub const MAX_HEADING_DIFF: f64 = 12.0 * std::f64::consts::PI / 180.0;
/// Sampling step along the lane (m).
const STEP: f64 = 2.0;

/// On which side of the travel direction a neighbour lies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// To the left.
    Left,
    /// To the right.
    Right,
}

/// A lane next to another, with the position mapping between them.
#[derive(Debug, Clone, PartialEq)]
pub struct Neighbour {
    /// The neighbouring lane.
    pub lane: LaneId,
    /// Its side relative to the lane it neighbours.
    pub side: Side,
    /// `(s on this lane, s on the neighbour, sideways distance to the neighbour (m, > 0))`
    /// every 2 m where the lanes run side by side.
    pub map: Vec<(f64, f64, f64)>,
}

impl Neighbour {
    /// The place on the neighbour next to `s` on this lane and the sideways distance there, if the
    /// lanes run side by side at `s` (a sample within 2.5 m).
    #[must_use]
    pub fn at(&self, s: f64) -> Option<(f64, f64)> {
        let i = self.map.partition_point(|m| m.0 < s);
        let mut best: Option<&(f64, f64, f64)> = None;
        for m in [i.checked_sub(1), Some(i)]
            .into_iter()
            .flatten()
            .filter_map(|k| self.map.get(k))
        {
            if best.is_none_or(|b| (m.0 - s).abs() < (b.0 - s).abs()) {
                best = Some(m);
            }
        }
        let m = best.filter(|m| (m.0 - s).abs() <= 2.5)?;
        // Interpolate along the lane so the mapping is smooth between samples.
        Some((m.1 + (s - m.0), m.2))
    }

    /// Whether the lanes run side by side from `s` to `s + ahead` on this lane.
    #[must_use]
    pub fn covers(&self, s: f64, ahead: f64) -> bool {
        let mut x = s;
        while x <= s + ahead {
            if self.at(x).is_none() {
                return false;
            }
            x += STEP;
        }
        true
    }
}

fn angle_between(a: f64, b: f64) -> f64 {
    let d = (a - b).rem_euclid(std::f64::consts::TAU);
    d.min(std::f64::consts::TAU - d)
}

impl LaneGraph {
    /// The lanes next to `id` (empty for unknown lanes), computed on first use.
    #[must_use]
    pub fn neighbours(&self, id: LaneId) -> &[Neighbour] {
        match self.neighbour_cache.get(id.0 as usize) {
            Some(cache) => cache.get_or_init(|| self.compute_neighbours(id)),
            None => &[],
        }
    }

    fn compute_neighbours(&self, id: LaneId) -> Vec<Neighbour> {
        let Some(lane) = self.lane(id) else {
            return Vec::new();
        };
        let g = &lane.geometry;
        let n = (g.length / STEP).ceil().max(1.0) as usize;
        let mine: Vec<(f64, DVec2, f64)> = (0..=n)
            .map(|i| {
                let s = (i as f64 * STEP).min(g.length);
                (s, g.point_at(s), g.heading_at(s))
            })
            .collect();
        let mut candidates = std::collections::BTreeSet::new();
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
        let mut out = Vec::new();
        for m in candidates {
            let other = LaneId(m);
            let Some(ol) = self.lane(other) else { continue };
            if ol.kind != lane.kind {
                continue;
            }
            let og = &ol.geometry;
            let on = (og.length / 1.0).ceil().max(1.0) as usize;
            let theirs: Vec<(f64, DVec2, f64)> = (0..=on)
                .map(|i| {
                    let s = (i as f64).min(og.length);
                    (s, og.point_at(s), og.heading_at(s))
                })
                .collect();
            let mut map = Vec::new();
            let (mut left, mut right) = (0usize, 0usize);
            for (s, p, h) in &mine {
                let rightv = DVec2::new(h.cos(), -h.sin());
                // The nearest sample of the other lane in front of / behind me, side by side.
                let hit = theirs
                    .iter()
                    .filter(|(_, _, oh)| angle_between(*h, *oh) <= MAX_HEADING_DIFF)
                    .map(|(os, q, _)| {
                        let d = *q - *p;
                        (*os, d.dot(rightv), d.dot(DVec2::new(h.sin(), h.cos())))
                    })
                    .filter(|(_, side, along)| {
                        along.abs() <= 1.0
                            && (MIN_SEPARATION..=MAX_SEPARATION).contains(&side.abs())
                    })
                    .min_by(|a, b| a.2.abs().total_cmp(&b.2.abs()));
                if let Some((os, side, _)) = hit {
                    map.push((*s, os, side.abs()));
                    if side > 0.0 {
                        right += 1;
                    } else {
                        left += 1;
                    }
                }
            }
            if map.len() * 2 < mine.len() {
                continue;
            }
            let side = if right >= left {
                Side::Right
            } else {
                Side::Left
            };
            out.push(Neighbour {
                lane: other,
                side,
                map,
            });
        }
        out
    }
}
