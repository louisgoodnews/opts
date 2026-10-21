//! The passenger rig: what the passengers of a vehicle chain are, as the game keeps them.

use std::collections::HashMap;

use bevy::math::Affine3A;
use bevy::prelude::*;
use opts_bevy::coords::omsi_to_bevy_pos;
use opts_core::formats::ticketpack::TicketPack;
use opts_core::formats::vehicles::cabin::PassengerCabin;
use opts_core::formats::vehicles::PassengerPaths;
use opts_sim::passengers::{chain, PassengerSim, Rng, TicketBook, Unit};

use super::{ticket_book, unit_spec};

/// How far from the door the people come to wait for it to open (m).
const DOOR_APPROACH_M: f32 = 0.9;

/// The passengers of a vehicle.
#[derive(Component)]
pub struct PassengerRig {
    /// The simulation of the chain.
    pub sim: PassengerSim,
    /// The units of the chain, head first, with the entity their people are parented to.
    pub(super) units: Vec<UnitSource>,
    pub(super) book: TicketBook,
    pub(super) seed: u64,
    pub(super) stop: Option<Entity>,
    pub(super) actors: HashMap<u32, Shown>,
    pub(super) approaching: Vec<Walker>,
    pub(super) leaving: Vec<Walker>,
    pub(super) rng: Rng,
    /// Visits of stops so far (for the sounds and the report).
    pub stops_served: u32,
    /// People to send to the doors now, stop or no stop (`vehicle.passengers spawn`).
    pub force_group: u32,
    /// The entry that group is sent to (its number in the chain).
    pub force_entry: usize,
}

/// A door as the scripts know it: the engine's entity and the door's number in it.
pub(super) type DoorSlot = (Entity, usize);

/// One unit of the chain: the entity of its frame and its cabin.
pub(super) struct UnitSource {
    pub(super) root: Entity,
    pub(super) unit: Unit,
    /// The entity whose script engine reports the unit's doors (`PAX_Entry<n>_Open`): the head,
    /// a unit with scripts of its own, or the unit whose scripts the unit shares.
    pub(super) store: Entity,
}

/// A passenger inside, as drawn.
pub(super) struct Shown {
    pub(super) entity: Entity,
    pub(super) unit: usize,
    pub(super) leg_m: f32,
    pub(super) phase: f32,
    pub(super) local: Vec3,
    pub(super) seated: Option<f32>,
}

/// A person outside: walking up to the bus or away from it.
pub(super) struct Walker {
    pub(super) entity: Entity,
    pub(super) leg_m: f32,
    pub(super) phase: f32,
    pub(super) position: Vec3,
    pub(super) target: Vec3,
    pub(super) entry: usize,
    pub(super) age: u32,
    pub(super) stops: u32,
    pub(super) walked_m: f32,
}

impl PassengerRig {
    /// A rig from the files of the head unit (`root` is its entity), `None` when the cabin has no
    /// entry or no pathpoints.
    #[must_use]
    pub fn from_files(
        cabin: &[u8],
        paths: &[u8],
        tickets: TicketPack,
        seed: u64,
        root: Entity,
    ) -> Option<Self> {
        let unit = unit_spec(
            &PassengerCabin::parse_bytes(cabin),
            &PassengerPaths::parse_bytes(paths).ok()?,
        );
        if unit.cabin.entries.is_empty() || unit.paths.points.is_empty() {
            return None;
        }
        let book = ticket_book(&tickets);
        let units = vec![UnitSource {
            root,
            unit,
            store: root,
        }];
        let sim = Self::sim_of(&units, &book, seed);
        Some(Self {
            sim,
            units,
            book,
            seed,
            stop: None,
            actors: HashMap::new(),
            approaching: Vec::new(),
            leaving: Vec::new(),
            rng: Rng::new(seed ^ 0xA5A5),
            stops_served: 0,
            force_group: 0,
            force_entry: 0,
        })
    }

    pub(super) fn sim_of(units: &[UnitSource], book: &TicketBook, seed: u64) -> PassengerSim {
        let specs: Vec<Unit> = units.iter().map(|u| u.unit.clone()).collect();
        let (cabin, paths) = chain(&specs);
        PassengerSim::new(cabin, &paths, book.clone(), seed)
    }

    /// Adds the unit behind the chain (its cabin joins the others'); nobody may be aboard yet.
    pub fn add_unit(&mut self, root: Entity, unit: Unit, store: Entity) {
        self.units.push(UnitSource { root, unit, store });
        self.sim = Self::sim_of(&self.units, &self.book, self.seed);
    }

    /// For every entry and every exit of the chain, in chain order: the entity whose engine
    /// reports the door and the door's number in that engine (`PAX_Entry<n>`; a store counts
    /// the doors of all the units that use it, in chain order).
    pub(super) fn door_slots(&self) -> (Vec<DoorSlot>, Vec<DoorSlot>) {
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
    pub(super) fn door_outside(&self, entry: usize) -> (usize, Vec3) {
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
    pub(super) fn door_world(&self, entry: usize, frames: &[Affine3A]) -> Vec3 {
        let (u, local) = self.door_outside(entry);
        frames
            .get(u)
            .or(frames.first())
            .map_or(local, |f| f.transform_point3(local))
    }

    /// The entry nearest to the world point `at`.
    pub(super) fn nearest_entry(&self, at: Vec3, frames: &[Affine3A]) -> usize {
        (0..self.sim.cabin.entries.len())
            .min_by(|&a, &b| {
                self.door_world(a, frames)
                    .distance(at)
                    .total_cmp(&self.door_world(b, frames).distance(at))
            })
            .unwrap_or(0)
    }
}
