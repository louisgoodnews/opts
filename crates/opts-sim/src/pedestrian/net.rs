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
    /// Lanes of each rail points object, by position (see [`super::switches`]).
    pub(super) gated: HashMap<u64, Vec<LaneId>>,
    /// The position each rail points object is set to (default 0).
    pub(super) positions: HashMap<u64, u8>,
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
            self.register_gate(id, &lane);
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
                self.unregister_gate(id, &lane);
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
    /// How far from the lane's centre line the walker keeps, to the right of the walking
    /// direction, as a fraction of half the lane width (see [`Walker::pose`]). `0` walks on the
    /// line.
    pub lateral: f32,
}

/// The range a new walker's [`Walker::lateral`] is drawn from: not on the centre line, not at the
/// very edge, so walkers in opposite directions pass at least `0.3` of a half-width apart.
const LATERAL_RANGE: (f32, f32) = (0.15, 0.85);

/// Walkers keep at most this far (m) from the centre line, however wide the lane is (a wide
/// lane is a plaza, not a pavement).
const MAX_OFFSET_M: f32 = 1.5;

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
            lateral: LATERAL_RANGE.0 + next_f32(rng) * (LATERAL_RANGE.1 - LATERAL_RANGE.0),
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
    ///
    /// The position is beside the lane's centre line, on the right of the walking direction by
    /// [`Walker::lateral`] times half the lane width (at most 1.5 m), so walkers in
    /// opposite directions pass on different sides. Walkers do not react to each other: two
    /// walking the same way at the same place and offset still overlap.
    #[must_use]
    pub fn pose(&self, net: &PedestrianNet) -> Option<(Vec3, f32)> {
        let lane = net.lane(self.lane)?;
        let (p, h) = lane.at(self.s);
        let heading = if self.dir > 0 {
            h
        } else {
            h + std::f32::consts::PI
        };
        let half = if lane.width.is_finite() {
            (lane.width * 0.5).clamp(0.0, MAX_OFFSET_M)
        } else {
            0.0
        };
        // Heading is clockwise from north with x east and y north: the right-hand side is
        // `(cos, -sin)`.
        let right = Vec3::new(heading.cos(), -heading.sin(), 0.0);
        Some((p + right * (self.lateral * half), heading))
    }
}

#[cfg(test)]
mod tests;
