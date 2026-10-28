//! Spatial queries, random/weighted spot selection, and shortest-path routing on [`LaneGraph`].

use super::*;

impl LaneGraph {
    /// The place on an accepted lane nearest to `point` among those that run within
    /// `max_heading_diff` radians of `heading` (the direction a vehicle standing there faces):
    /// the lane, the distance along it and the distance from `point` (m). Found by sampling the
    /// lane every metre; `None` when no such lane lies within `max_dist`.
    #[must_use]
    pub fn nearest_place(
        &self,
        point: DVec2,
        heading: f64,
        max_heading_diff: f64,
        max_dist: f64,
        accept: impl Fn(&Lane) -> bool,
    ) -> Option<(LaneId, f64, f64)> {
        let mut best: Option<(LaneId, f64, f64)> = None;
        for (i, lane) in self.lanes.iter().enumerate() {
            if chord_distance(lane, point) > max_dist + 0.25 * lane.length() || !accept(lane) {
                continue;
            }
            let len = lane.length();
            let mut s = 0.0;
            while s <= len + 1e-9 {
                let d = lane.geometry.point_at(s).distance(point);
                let mut diff =
                    (lane.geometry.heading_at(s) - heading).rem_euclid(std::f64::consts::TAU);
                if diff > std::f64::consts::PI {
                    diff = std::f64::consts::TAU - diff;
                }
                if d <= max_dist && diff <= max_heading_diff && best.is_none_or(|b| d < b.2) {
                    best = Some((LaneId(i as u32), s.min(len), d));
                }
                s += 1.0;
            }
        }
        best
    }

    /// A random place on an accepted lane at a distance of `min..=max` metres from `centre`
    /// (up to 12 tries). `accept` filters lanes, e.g. by kind and density rule.
    pub fn random_spot(
        &self,
        centre: DVec2,
        min: f64,
        max: f64,
        rng: &mut Rng,
        accept: impl Fn(&Lane) -> bool,
    ) -> Option<(LaneId, f64)> {
        // Lanes with a start or end within reach; lane lengths are short against the radius.
        let near: Vec<u32> = self
            .lanes
            .iter()
            .enumerate()
            .filter(|(_, l)| {
                // Distance to the chord, with slack for the bulge of an arc.
                chord_distance(l, centre) <= max + 0.25 * l.length() && accept(l)
            })
            .map(|(i, _)| i as u32)
            .collect();
        if near.is_empty() {
            return None;
        }
        for _ in 0..12 {
            let id = LaneId(near[rng.below(near.len())]);
            let lane = &self.lanes[id.0 as usize];
            let s = rng.next_f64() * lane.length();
            let d = lane.geometry.point_at(s).distance(centre);
            if (min..=max).contains(&d) {
                return Some((id, s));
            }
        }
        None
    }

    /// A random place on a lane at a distance of `min..=max` metres from `centre`, the lane drawn
    /// with probability proportional to `weight` (lanes weighing `0` or less are never drawn; up
    /// to 12 tries). Spawning uses `length × density` as the weight.
    pub fn weighted_spot(
        &self,
        centre: DVec2,
        min: f64,
        max: f64,
        rng: &mut Rng,
        weight: impl Fn(&Lane) -> f64,
    ) -> Option<(LaneId, f64)> {
        let mut near: Vec<(u32, f64)> = Vec::new();
        let mut total = 0.0;
        for (i, l) in self.lanes.iter().enumerate() {
            if chord_distance(l, centre) > max + 0.25 * l.length() {
                continue;
            }
            let w = weight(l);
            if w > 0.0 && w.is_finite() {
                total += w;
                near.push((i as u32, total));
            }
        }
        if near.is_empty() {
            return None;
        }
        for _ in 0..12 {
            let pick = rng.next_f64() * total;
            let at = near
                .partition_point(|&(_, acc)| acc <= pick)
                .min(near.len() - 1);
            let id = LaneId(near[at].0);
            let lane = &self.lanes[id.0 as usize];
            let s = rng.next_f64() * lane.length();
            let d = lane.geometry.point_at(s).distance(centre);
            if (min..=max).contains(&d) {
                return Some((id, s));
            }
        }
        None
    }

    /// Shortest route (by length) from `from` to `to`, both included.
    #[must_use]
    pub fn shortest_route(&self, from: LaneId, to: LaneId) -> Option<Vec<LaneId>> {
        #[derive(PartialEq)]
        struct Item(f64, u32);
        impl Eq for Item {}
        impl Ord for Item {
            fn cmp(&self, o: &Self) -> Ordering {
                o.0.total_cmp(&self.0).then(o.1.cmp(&self.1))
            }
        }
        impl PartialOrd for Item {
            fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
                Some(self.cmp(o))
            }
        }
        self.lane(from)?;
        self.lane(to)?;
        let mut dist = vec![f64::INFINITY; self.lanes.len()];
        let mut prev = vec![u32::MAX; self.lanes.len()];
        let mut heap = BinaryHeap::new();
        dist[from.0 as usize] = self.lanes[from.0 as usize].length();
        heap.push(Item(dist[from.0 as usize], from.0));
        while let Some(Item(d, u)) = heap.pop() {
            if u == to.0 {
                let mut route = vec![to];
                let mut cur = to.0;
                while prev[cur as usize] != u32::MAX {
                    cur = prev[cur as usize];
                    route.push(LaneId(cur));
                }
                route.reverse();
                return Some(route);
            }
            if d > dist[u as usize] {
                continue;
            }
            for &LaneId(v) in &self.successors[u as usize] {
                let nd = d + self.lanes[v as usize].length();
                if nd < dist[v as usize] {
                    dist[v as usize] = nd;
                    prev[v as usize] = u;
                    heap.push(Item(nd, v));
                }
            }
        }
        None
    }
}

/// Distance from `centre` to the chord between a lane's ends.
fn chord_distance(l: &Lane, centre: DVec2) -> f64 {
    let (a, b) = (l.geometry.start, l.geometry.end());
    let ab = b - a;
    let t = if ab.length_squared() > 1e-9 {
        ((centre - a).dot(ab) / ab.length_squared()).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (a + ab * t).distance(centre)
}
