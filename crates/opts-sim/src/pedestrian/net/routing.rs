//! routes of the net: continuations of a lane end and the shortest path.

use super::*;

impl PedestrianNet {
    /// Lanes that continue where `lane` ends at `end` and may be entered there.
    #[must_use]
    pub fn continuations(&self, lane: LaneId, end: End) -> Vec<(LaneId, End)> {
        let Some(l) = self.lanes.get(&lane) else {
            return Vec::new();
        };
        let p = l.end_point(end == End::End);
        let (cx, cy) = cell(p);
        let mut out = Vec::new();
        for dx in -1..=1 {
            for dy in -1..=1 {
                for &(id, e) in self.ends.get(&(cx + dx, cy + dy)).into_iter().flatten() {
                    if id == lane {
                        continue;
                    }
                    let Some(other) = self.lanes.get(&id) else {
                        continue;
                    };
                    let q = other.end_point(e == End::End);
                    let near = (q.x - p.x).hypot(q.y - p.y) <= CONNECT_XY_M
                        && (q.z - p.z).abs() <= CONNECT_Z_M;
                    // Entering at the start walks forward, entering at the end walks backward.
                    let enterable = match e {
                        End::Start => matches!(other.travel, Travel::Forward | Travel::Both),
                        End::End => matches!(other.travel, Travel::Reverse | Travel::Both),
                    };
                    if near && enterable {
                        out.push((id, e));
                    }
                }
            }
        }
        out.sort_unstable_by_key(|(id, e)| (*id, *e == End::End));
        out
    }

    /// The shortest way over the lanes from the head on `from` walking in direction `dir`
    /// (`+1` towards the lane's end) to the lane `to`: the lanes after `from` up to and
    /// including `to`, each with the direction it is walked in. Empty when `from == to`; `None`
    /// when `to` cannot be reached (or lies beyond `limit_m` of track).
    #[must_use]
    pub fn shortest_path(
        &self,
        from: LaneId,
        dir: i8,
        to: LaneId,
        limit_m: f32,
    ) -> Option<Vec<(LaneId, i8)>> {
        use std::cmp::Reverse;
        use std::collections::{BinaryHeap, HashMap};
        if from == to {
            return Some(Vec::new());
        }
        let start = (from, dir.signum());
        let mut best: HashMap<(LaneId, i8), f32> = HashMap::new();
        let mut prev: HashMap<(LaneId, i8), (LaneId, i8)> = HashMap::new();
        // Distances in millimetres keep the heap ordered without float keys.
        let mut heap = BinaryHeap::new();
        best.insert(start, 0.0);
        heap.push(Reverse((0_u64, start.0, start.1)));
        while let Some(Reverse((mm, lane, d))) = heap.pop() {
            let cost = mm as f32 / 1000.0;
            if best.get(&(lane, d)).is_some_and(|b| *b < cost) {
                continue;
            }
            if lane == to {
                let mut out = vec![(lane, d)];
                let mut at = (lane, d);
                while let Some(p) = prev.get(&at) {
                    if *p == start {
                        break;
                    }
                    out.push(*p);
                    at = *p;
                }
                out.reverse();
                return Some(out);
            }
            let Some(l) = self.lanes.get(&lane) else {
                continue;
            };
            let next_cost = cost + l.length();
            if next_cost > limit_m {
                continue;
            }
            let end = if d > 0 { End::End } else { End::Start };
            for (next, entry) in self.continuations(lane, end) {
                let nd = if entry == End::Start { 1 } else { -1 };
                let key = (next, nd);
                if best.get(&key).is_none_or(|b| next_cost < *b) {
                    best.insert(key, next_cost);
                    prev.insert(key, (lane, d));
                    heap.push(Reverse(((next_cost * 1000.0) as u64, next, nd)));
                }
            }
        }
        None
    }
}
