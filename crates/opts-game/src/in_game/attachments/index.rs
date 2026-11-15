//! The map-wide index of splines and their chain offsets, which places attachments on chains.

use std::collections::{HashMap, HashSet};

use super::*;

/// Every spline of a map by id code, with the tile it lies on and its chain offset.
#[derive(Default)]
pub struct SplineIndex {
    pub(super) by_id: HashMap<u64, Placed>,
    /// Total length of the splines preceding each spline in its chain (following `conn_id_1`).
    offsets: HashMap<u64, f64>,
}

impl SplineIndex {
    /// Adds the splines of one tile. Call [`Self::finish`] after the last tile.
    pub fn add_tile(&mut self, key: (i32, i32), splines: &[MapSplineInstance]) {
        for s in splines {
            self.by_id.insert(s.id_code, (key, s.clone()));
        }
        self.offsets.clear();
    }

    /// Number of splines indexed.
    #[must_use]
    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    /// Whether no spline is indexed.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }

    /// Computes the chain offsets of all splines (each chain is walked once).
    pub fn finish(&mut self) {
        let ids: Vec<u64> = self.by_id.keys().copied().collect();
        let mut offsets: HashMap<u64, f64> = HashMap::with_capacity(ids.len());
        for id in ids {
            if offsets.contains_key(&id) {
                continue;
            }
            // Walk back to the chain's head or to a spline whose offset is known.
            let mut path = vec![id];
            let mut seen = HashSet::from([id]);
            let mut base = Some(0.0);
            loop {
                let cur = *path.last().unwrap_or(&id);
                let prev = self.by_id.get(&cur).map_or(0, |(_, s)| s.conn_id_1);
                let Some((_, p)) = self.by_id.get(&prev) else {
                    break;
                };
                if let Some(o) = offsets.get(&prev) {
                    base = Some(o + f64::from(p.length_m));
                    break;
                }
                if !seen.insert(prev) || path.len() >= MAX_CHAIN {
                    // A loop (or an absurdly long chain): walk each spline on its own.
                    base = None;
                    break;
                }
                path.push(prev);
            }
            match base {
                Some(mut o) => {
                    for (i, &sid) in path.iter().enumerate().rev() {
                        if i + 1 < path.len() {
                            o += self
                                .by_id
                                .get(&path[i + 1])
                                .map_or(0.0, |(_, s)| f64::from(s.length_m));
                        }
                        offsets.insert(sid, o);
                    }
                }
                None => {
                    for &sid in &path {
                        offsets.insert(sid, self.walk_offset(sid));
                    }
                }
            }
        }
        self.offsets = offsets;
    }

    /// Chain offset by walking `conn_id_1` until the head or a spline already passed.
    pub(super) fn walk_offset(&self, id: u64) -> f64 {
        let mut offset = 0.0;
        let mut seen = HashSet::from([id]);
        let mut cur = id;
        while let Some((_, s)) = self.by_id.get(&cur) {
            let prev = s.conn_id_1;
            let Some((_, p)) = self.by_id.get(&prev) else {
                break;
            };
            if !seen.insert(prev) || seen.len() > MAX_CHAIN {
                break;
            }
            offset += f64::from(p.length_m);
            cur = prev;
        }
        offset
    }

    /// Total length of the splines preceding `id` in its chain.
    #[must_use]
    pub fn chain_offset(&self, id: u64) -> f64 {
        self.offsets
            .get(&id)
            .copied()
            .unwrap_or_else(|| self.walk_offset(id))
    }

    /// Resolves arc length `s` on spline `id`, walking into neighbouring splines when it lies
    /// outside the segment. Returns the spline, its tile and the local arc length.
    pub(super) fn locate(&self, id: u64, mut s: f64) -> Option<(&Placed, f64)> {
        let mut cur = self.by_id.get(&id)?;
        for _ in 0..MAX_CHAIN {
            let len = f64::from(cur.1.length_m);
            if s < -CHAIN_EPS_M {
                let prev = self.by_id.get(&cur.1.conn_id_1)?;
                s += f64::from(prev.1.length_m);
                cur = prev;
            } else if s > len + CHAIN_EPS_M {
                let next = self.by_id.get(&cur.1.conn_id_2)?;
                s -= len;
                cur = next;
            } else {
                return Some((cur, s.clamp(0.0, len)));
            }
        }
        None
    }
}
