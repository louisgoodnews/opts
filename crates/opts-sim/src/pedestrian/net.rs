//! The footpath network of the loaded tiles and the walkers moving on it.

use std::collections::HashMap;

use glam::Vec3;

use super::lane::{FootLane, Travel};

/// Identifier of a lane in the network.
pub type LaneId = u32;

/// Horizontal distance within which two lane ends are one junction (m).
const CONNECT_XY_M: f32 = 0.6;
/// Vertical distance within which two lane ends are one junction (m).
const CONNECT_Z_M: f32 = 0.8;

/// Start or end of a lane.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum End {
    /// The first vertex.
    Start,
    /// The last vertex.
    End,
}

/// All footpath lanes of the resident tiles.
#[derive(Debug, Default)]
pub struct PedestrianNet {
    lanes: HashMap<LaneId, FootLane>,
    by_tile: HashMap<(i32, i32), Vec<LaneId>>,
    /// Lane ends by 1 m cell.
    ends: HashMap<(i32, i32), Vec<(LaneId, End)>>,
    next: LaneId,
}

fn cell(p: Vec3) -> (i32, i32) {
    (p.x.floor() as i32, p.y.floor() as i32)
}

impl PedestrianNet {
    /// Registers the lanes of a tile (replacing an earlier registration of the same tile).
    pub fn insert_tile(&mut self, tile: (i32, i32), lanes: Vec<FootLane>) {
        self.remove_tile(tile);
        let mut ids = Vec::with_capacity(lanes.len());
        for lane in lanes {
            let id = self.next;
            self.next += 1;
            for end in [End::Start, End::End] {
                let p = lane.end_point(end == End::End);
                self.ends.entry(cell(p)).or_default().push((id, end));
            }
            self.lanes.insert(id, lane);
            ids.push(id);
        }
        self.by_tile.insert(tile, ids);
    }

    /// Removes the lanes of a tile.
    pub fn remove_tile(&mut self, tile: (i32, i32)) {
        let Some(ids) = self.by_tile.remove(&tile) else {
            return;
        };
        for id in ids {
            if let Some(lane) = self.lanes.remove(&id) {
                for end in [End::Start, End::End] {
                    let c = cell(lane.end_point(end == End::End));
                    if let Some(list) = self.ends.get_mut(&c) {
                        list.retain(|(l, _)| *l != id);
                        if list.is_empty() {
                            self.ends.remove(&c);
                        }
                    }
                }
            }
        }
    }

    /// A lane by id.
    #[must_use]
    pub fn lane(&self, id: LaneId) -> Option<&FootLane> {
        self.lanes.get(&id)
    }

    /// The lane point nearest to `p` within `max_xy` metres horizontally: `(lane, arc length,
    /// distance)`. Lanes are scanned linearly (use it for placement, not per frame).
    #[must_use]
    pub fn nearest(&self, p: Vec3, max_xy: f32) -> Option<(LaneId, f32, f32)> {
        let mut best: Option<(LaneId, f32, f32)> = None;
        for (&id, lane) in &self.lanes {
            for w in lane.points.windows(2).zip(lane.cum.windows(2)) {
                let (pts, cum) = w;
                let (a, b) = (pts[0], pts[1]);
                let ab = b - a;
                let l2 = (ab.x * ab.x + ab.y * ab.y).max(1e-9);
                let t = (((p.x - a.x) * ab.x + (p.y - a.y) * ab.y) / l2).clamp(0.0, 1.0);
                let q = a + ab * t;
                let d = (q.x - p.x).hypot(q.y - p.y);
                if d <= max_xy && best.is_none_or(|b| d < b.2 || (d == b.2 && id < b.0)) {
                    best = Some((id, cum[0] + (cum[1] - cum[0]) * t, d));
                }
            }
        }
        best
    }

    /// Number of lanes.
    #[must_use]
    pub fn lane_count(&self) -> usize {
        self.lanes.len()
    }

    /// Sum of all lane lengths (m).
    #[must_use]
    pub fn total_length(&self) -> f32 {
        self.lanes.values().map(FootLane::length).sum()
    }

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

    /// A random point on a lane at a horizontal distance of `min_m..max_m` from `centre`.
    pub fn random_spot(
        &self,
        centre: (f32, f32),
        min_m: f32,
        max_m: f32,
        rng: &mut u32,
    ) -> Option<(LaneId, f32)> {
        let near: Vec<(LaneId, f32)> = self
            .lanes
            .iter()
            .filter(|(_, l)| {
                let (p, _) = l.at(l.length() * 0.5);
                (p.x - centre.0).hypot(p.y - centre.1) <= max_m + l.length() * 0.5
            })
            .map(|(id, l)| (*id, l.length()))
            .collect();
        let total: f32 = near.iter().map(|(_, len)| *len).sum();
        if total <= 0.0 {
            return None;
        }
        let mut near = near;
        near.sort_by_key(|(id, _)| *id); // HashMap order must not leak into the result
        for _ in 0..24 {
            let mut pick = next_f32(rng) * total;
            for (id, len) in &near {
                if pick <= *len {
                    let s = (pick / len.max(1e-3)).clamp(0.0, 1.0) * len;
                    let (p, _) = self.lanes[id].at(s);
                    let d = (p.x - centre.0).hypot(p.y - centre.1);
                    if (min_m..=max_m).contains(&d) {
                        return Some((*id, s));
                    }
                    break;
                }
                pick -= len;
            }
        }
        None
    }
}

/// xorshift32 step returning a value in `[0, 1)`.
pub fn next_f32(state: &mut u32) -> f32 {
    *state ^= *state << 13;
    *state ^= *state >> 17;
    *state ^= *state << 5;
    (*state >> 8) as f32 / (1u32 << 24) as f32
}

/// Result of [`Walker::advance`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Advance {
    /// The walker is on a lane.
    Moving,
    /// The lane ended in a one-way dead end or disappeared (its tile was unloaded).
    Finished,
}

/// A pedestrian on the network.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Walker {
    /// Current lane.
    pub lane: LaneId,
    /// Arc length on the lane (m).
    pub s: f32,
    /// `+1` walking towards the end, `-1` towards the start.
    pub dir: i8,
    /// Walking speed (m/s).
    pub speed: f32,
    /// Gait phase (rad); advanced by the caller from the speed and the stride.
    pub phase: f32,
}

impl Walker {
    /// Places a walker on `lane` at `s`, choosing an allowed direction.
    #[must_use]
    pub fn new(
        net: &PedestrianNet,
        lane: LaneId,
        s: f32,
        speed: f32,
        rng: &mut u32,
    ) -> Option<Self> {
        let dir = match net.lane(lane)?.travel {
            Travel::Forward => 1,
            Travel::Reverse => -1,
            Travel::Both => {
                if next_f32(rng) < 0.5 {
                    1
                } else {
                    -1
                }
            }
        };
        Some(Self {
            lane,
            s,
            dir,
            speed,
            phase: next_f32(rng) * std::f32::consts::TAU,
        })
    }

    /// Moves the walker `dt` seconds along the network.
    pub fn advance(&mut self, dt: f32, net: &PedestrianNet, rng: &mut u32) -> Advance {
        let mut left = self.speed * dt;
        // A long step may cross several short lanes; bounded to stay finite.
        for _ in 0..8 {
            let Some(lane) = net.lane(self.lane) else {
                return Advance::Finished;
            };
            let len = lane.length();
            let room = if self.dir > 0 { len - self.s } else { self.s };
            if left <= room {
                self.s += f32::from(self.dir) * left;
                return Advance::Moving;
            }
            left -= room.max(0.0);
            let end = if self.dir > 0 { End::End } else { End::Start };
            self.s = if self.dir > 0 { len } else { 0.0 };
            let next = net.continuations(self.lane, end);
            if next.is_empty() {
                let back_ok = match (lane.travel, self.dir) {
                    (Travel::Both, _) => true,
                    (Travel::Forward, d) if d < 0 => true,
                    (Travel::Reverse, d) if d > 0 => true,
                    _ => false,
                };
                // Turning round is only allowed on a lane that may be walked in both ways.
                if lane.travel == Travel::Both && back_ok {
                    self.dir = -self.dir;
                    continue;
                }
                return Advance::Finished;
            }
            let (id, entry) = next[(next_f32(rng) * next.len() as f32) as usize % next.len()];
            self.lane = id;
            match entry {
                End::Start => {
                    self.dir = 1;
                    self.s = 0.0;
                }
                End::End => {
                    self.dir = -1;
                    self.s = net.lane(id).map_or(0.0, FootLane::length);
                }
            }
        }
        Advance::Moving
    }

    /// World position and heading (radians, clockwise from north, in the walking direction).
    #[must_use]
    pub fn pose(&self, net: &PedestrianNet) -> Option<(Vec3, f32)> {
        let (p, h) = net.lane(self.lane)?.at(self.s);
        Some((
            p,
            if self.dir > 0 {
                h
            } else {
                h + std::f32::consts::PI
            },
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lane(a: Vec3, b: Vec3, travel: Travel) -> FootLane {
        FootLane::new(vec![a, b], travel, 1.0).unwrap()
    }

    fn net() -> PedestrianNet {
        let mut n = PedestrianNet::default();
        // Two bidirectional lanes meeting at (0, 10), a one-way dead end after (0, 20).
        n.insert_tile(
            (0, 0),
            vec![
                lane(Vec3::ZERO, Vec3::new(0.0, 10.0, 0.0), Travel::Both),
                lane(
                    Vec3::new(0.0, 10.0, 0.2),
                    Vec3::new(0.0, 20.0, 0.2),
                    Travel::Both,
                ),
                lane(
                    Vec3::new(0.0, 20.0, 0.2),
                    Vec3::new(0.0, 30.0, 0.2),
                    Travel::Forward,
                ),
            ],
        );
        n
    }

    #[test]
    fn continuations_respect_geometry_and_direction() {
        let n = net();
        assert_eq!(n.continuations(0, End::End), vec![(1, End::Start)]);
        assert_eq!(n.continuations(1, End::Start), vec![(0, End::End)]);
        // Lane 2 is forward-only: entering at its end is not allowed.
        assert_eq!(n.continuations(1, End::End), vec![(2, End::Start)]);
        assert!(n.continuations(2, End::End).is_empty());
    }

    #[test]
    fn walker_crosses_lanes_turns_round_and_finishes_at_a_one_way_end() {
        let n = net();
        let mut rng = 7;
        let mut w = Walker {
            lane: 0,
            s: 5.0,
            dir: 1,
            speed: 1.0,
            phase: 0.0,
        };
        assert_eq!(w.advance(8.0, &n, &mut rng), Advance::Moving);
        assert_eq!(w.lane, 1);
        assert!((w.s - 3.0).abs() < 1e-4);
        // Run to the forward-only lane and past its end: a one-way lane cannot be turned on.
        assert_eq!(w.advance(30.0, &n, &mut rng), Advance::Finished);
    }

    #[test]
    fn dead_end_of_a_two_way_lane_turns_the_walker_round() {
        let mut n = PedestrianNet::default();
        n.insert_tile(
            (0, 0),
            vec![lane(Vec3::ZERO, Vec3::new(0.0, 10.0, 0.0), Travel::Both)],
        );
        let mut rng = 1;
        let mut w = Walker {
            lane: 0,
            s: 9.0,
            dir: 1,
            speed: 1.0,
            phase: 0.0,
        };
        assert_eq!(w.advance(3.0, &n, &mut rng), Advance::Moving);
        assert_eq!(w.dir, -1);
        assert!((w.s - 8.0).abs() < 1e-4);
    }

    #[test]
    fn nearest_finds_the_closest_lane_point() {
        let n = net();
        let (lane, s, d) = n.nearest(Vec3::new(2.0, 4.0, 0.0), 10.0).unwrap();
        assert_eq!(lane, 0);
        assert!((s - 4.0).abs() < 1e-4 && (d - 2.0).abs() < 1e-4);
        assert!(n.nearest(Vec3::new(50.0, 4.0, 0.0), 10.0).is_none());
    }

    #[test]
    fn removing_a_tile_removes_its_lanes_and_links() {
        let mut n = net();
        n.remove_tile((0, 0));
        assert_eq!(n.lane_count(), 0);
        assert!(n.continuations(0, End::End).is_empty());
        let mut w = Walker {
            lane: 0,
            s: 1.0,
            dir: 1,
            speed: 1.0,
            phase: 0.0,
        };
        assert_eq!(w.advance(1.0, &n, &mut 3), Advance::Finished);
    }

    #[test]
    fn spawn_spots_lie_in_the_requested_ring() {
        let n = net();
        let mut rng = 99;
        for _ in 0..50 {
            if let Some((id, s)) = n.random_spot((0.0, 0.0), 8.0, 22.0, &mut rng) {
                let (p, _) = n.lane(id).unwrap().at(s);
                let d = p.x.hypot(p.y);
                assert!((8.0..=22.0).contains(&d), "{d}");
            }
        }
    }
}
