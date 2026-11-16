//! The door geometry of the passenger rig: the entries and exits of the chain in the order the
//! scripts number them, the point in front of each entry where people wait, and the nearest entry.

use std::collections::HashMap;

use bevy::math::Affine3A;
use bevy::prelude::*;
use opts_bevy::coords::omsi_to_bevy_pos;

use super::{DoorSlot, PassengerRig};

/// How far from the door the people come to wait for it to open (m).
const DOOR_APPROACH_M: f32 = 0.9;

impl PassengerRig {
    /// For every entry and every exit of the chain, in chain order: the entity whose engine
    /// reports the door and the door's number in that engine (`PAX_Entry<n>`; a store counts
    /// the doors of all the units that use it, in chain order).
    pub(in crate::passengers) fn door_slots(&self) -> (Vec<DoorSlot>, Vec<DoorSlot>) {
        let mut counted: HashMap<Entity, (usize, usize)> = HashMap::new();
        let (mut entries, mut exits) = (Vec::new(), Vec::new());
        for u in &self.units {
            let c = counted.entry(u.store).or_default();
            for _ in &u.unit.cabin.entries {
                entries.push((u.store, c.0));
                c.0 += 1;
            }
            for _ in &u.unit.cabin.exits {
                exits.push((u.store, c.1));
                c.1 += 1;
            }
        }
        (entries, exits)
    }

    /// The point in front of entry `entry` where people wait: the unit it is in and the point in
    /// that unit's frame (Bevy axes).
    pub(in crate::passengers) fn door_outside(&self, entry: usize) -> (usize, Vec3) {
        let node = self.sim.cabin.entries.get(entry).copied().unwrap_or(0);
        let p = self
            .sim
            .graph()
            .points
            .get(node)
            .copied()
            .unwrap_or(Vec3::ZERO);
        let out = if p.x >= 0.0 { 1.0 } else { -1.0 };
        (
            self.sim.graph().unit(node),
            omsi_to_bevy_pos(p + Vec3::new(out * DOOR_APPROACH_M, 0.0, 0.0)),
        )
    }

    /// World position of the point in front of entry `entry`, `frames` being the world frames of
    /// the units.
    pub(in crate::passengers) fn door_world(&self, entry: usize, frames: &[Affine3A]) -> Vec3 {
        let (u, local) = self.door_outside(entry);
        frames
            .get(u)
            .or(frames.first())
            .map_or(local, |f| f.transform_point3(local))
    }

    /// The entry nearest to the world point `at`.
    pub(in crate::passengers) fn nearest_entry(&self, at: Vec3, frames: &[Affine3A]) -> usize {
        (0..self.sim.cabin.entries.len())
            .min_by(|&a, &b| {
                self.door_world(a, frames)
                    .distance(at)
                    .total_cmp(&self.door_world(b, frames).distance(at))
            })
            .unwrap_or(0)
    }
}
