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

}

mod routing;
mod spawn;
mod walker;

pub use spawn::next_f32;
pub use walker::{Advance, Walker};

#[cfg(test)]
mod tests;
