//! Rail points on the track network: lanes that are passable only in one position of their
//! points (`[switchdir]` of the scenery object), and the position each points object is in.
//!
//! The network keeps the positions; the game sets them (a train that approaches asks for the
//! way it wants, a click toggles). Trains choose among the lanes that are open ([`Train`]
//! avoids a closed lane while an open one exists), so a switch in the wrong position is taken
//! as a trailing movement instead of leaving the rails.
//!
//! [`Train`]: crate::rail::Train

use super::lane::FootLane;
use super::net::{LaneId, PedestrianNet};

impl PedestrianNet {
    pub(super) fn register_gate(&mut self, id: LaneId, lane: &FootLane) {
        if let Some(g) = lane.gate {
            self.gated.entry(g.switch).or_default().push(id);
        }
    }

    pub(super) fn unregister_gate(&mut self, id: LaneId, lane: &FootLane) {
        if let Some(g) = lane.gate {
            if let Some(list) = self.gated.get_mut(&g.switch) {
                list.retain(|l| *l != id);
                if list.is_empty() {
                    self.gated.remove(&g.switch);
                }
            }
        }
    }

    /// The position `switch` is set to (0 until it is set).
    #[must_use]
    pub fn switch_position(&self, switch: u64) -> u8 {
        self.positions.get(&switch).copied().unwrap_or(0)
    }

    /// Sets the position of `switch`. The caller makes sure no vehicle is on the points.
    pub fn set_switch_position(&mut self, switch: u64, position: u8) {
        self.positions.insert(switch, position);
    }

    /// Whether `lane` belongs to points that are set to another position.
    #[must_use]
    pub fn is_closed(&self, lane: LaneId) -> bool {
        self.lane(lane)
            .and_then(|l| l.gate)
            .is_some_and(|g| self.switch_position(g.switch) != g.position)
    }

    /// The points `lane` belongs to, if it is gated.
    #[must_use]
    pub fn switch_of(&self, lane: LaneId) -> Option<u64> {
        self.lane(lane).and_then(|l| l.gate).map(|g| g.switch)
    }

    /// The lanes of the points `switch` (every position).
    #[must_use]
    pub fn switch_lanes(&self, switch: u64) -> &[LaneId] {
        self.gated.get(&switch).map_or(&[], Vec::as_slice)
    }

    /// The keys of all points of the loaded tiles.
    pub fn switches(&self) -> impl Iterator<Item = u64> + '_ {
        self.gated.keys().copied()
    }
}

/// The key of the points object placed at `(x, y)` (session frame, metres), to 0.125 m.
#[must_use]
pub fn switch_key(x: f32, y: f32) -> u64 {
    let q = |v: f32| u64::from((v * 8.0).round() as i32 as u32);
    q(x) << 32 | q(y)
}

/// The place a key was made from ([`switch_key`]).
#[must_use]
pub fn switch_place(key: u64) -> (f32, f32) {
    let q = |v: u64| (v as u32 as i32) as f32 / 8.0;
    (q(key >> 32), q(key))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pedestrian::{Gate, Travel};
    use glam::Vec3;

    fn lane(y: f32, gate: Option<Gate>) -> FootLane {
        let l = FootLane::new(
            vec![Vec3::new(0.0, y, 0.0), Vec3::new(0.0, y + 10.0, 0.0)],
            Travel::Both,
            1.0,
        )
        .unwrap();
        gate.map_or(l.clone(), |g| l.with_gate(g))
    }

    #[test]
    fn a_key_returns_its_place() {
        let (x, y) = switch_place(switch_key(-1234.5, 87.25));
        assert_eq!((x, y), (-1234.5, 87.25));
    }

    #[test]
    fn lanes_close_and_open_with_the_position_and_survive_a_tile_reload() {
        let key = switch_key(0.0, 0.0);
        let gate = |position| {
            Some(Gate {
                switch: key,
                position,
            })
        };
        let mut net = PedestrianNet::default();
        net.insert_tile(
            (0, 0),
            vec![lane(0.0, None), lane(20.0, gate(0)), lane(40.0, gate(1))],
        );
        let ids = net.switch_lanes(key).to_vec();
        assert_eq!(ids.len(), 2);
        assert!(!net.is_closed(ids[0]) && net.is_closed(ids[1]));
        net.set_switch_position(key, 1);
        assert!(net.is_closed(ids[0]) && !net.is_closed(ids[1]));
        net.insert_tile((0, 0), vec![lane(20.0, gate(0)), lane(40.0, gate(1))]);
        let again = net.switch_lanes(key).to_vec();
        assert_eq!(again.len(), 2, "the old registration is gone");
        assert_eq!(net.switch_position(key), 1, "the position is kept");
        net.remove_tile((0, 0));
        assert!(net.switch_lanes(key).is_empty());
    }
}
