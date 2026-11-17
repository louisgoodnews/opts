//! The passenger rig: what the passengers of a vehicle chain are, as the game keeps them.

use std::collections::HashMap;
use std::sync::Arc;

use bevy::prelude::*;
use opts_core::formats::ticketpack::TicketPack;
use opts_core::formats::vehicles::cabin::PassengerCabin;
use opts_core::formats::vehicles::PassengerPaths;
use opts_sim::passengers::{
    chain, Destination, DestinationTable, PassengerSim, Rng, TicketBook, Unit,
};

use super::{ticket_book, unit_spec};

mod doors;

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
    /// The stop being served, by position (key of [`super::StopCrowds`]).
    pub(super) stop_key: Option<(i32, i32)>,
    /// People at the stop beyond the drawn ones: they board without a figure.
    pub(super) unseen: Vec<Unseen>,
    /// Seconds since the last unseen person boarded.
    pub(super) unseen_timer: f32,
    /// People to send to the doors now, stop or no stop (`vehicle.passengers spawn`).
    pub force_group: u32,
    /// The entry that group is sent to (its number in the chain).
    pub force_entry: usize,
    /// The destinations of the map's stops, built from its timetables on first use.
    pub(super) destinations: Option<Arc<DestinationTable>>,
    /// The ID code of the stop the bus stands at (key of the destinations).
    pub(super) stop_id: Option<u64>,
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
    pub(super) seated: Option<SeatPose>,
}

/// The seat a passenger sits on, as the figure needs it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct SeatPose {
    /// Rotation about the vertical (rad, `[passpos]` `rot`).
    pub(super) rotation_rad: f32,
    /// Height of the seat point above the floor (m, `[passpos]` `h`).
    pub(super) height_m: f32,
}

/// A person at the stop who is not drawn (the crowd beyond [`super::MAX_VISIBLE_WAITING`]).
#[derive(Debug, Clone)]
pub(super) struct Unseen {
    pub(super) entry: usize,
    pub(super) age: u32,
    pub(super) stops: u32,
    pub(super) dest: Option<Destination>,
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
    pub(super) dest: Option<Destination>,
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
            stop_key: None,
            unseen: Vec::new(),
            unseen_timer: 0.0,
            actors: HashMap::new(),
            approaching: Vec::new(),
            leaving: Vec::new(),
            rng: Rng::new(seed ^ 0xA5A5),
            stops_served: 0,
            force_group: 0,
            force_entry: 0,
            destinations: None,
            stop_id: None,
        })
    }

    pub(super) fn sim_of(units: &[UnitSource], book: &TicketBook, seed: u64) -> PassengerSim {
        let specs: Vec<Unit> = units.iter().map(|u| u.unit.clone()).collect();
        let (cabin, paths) = chain(&specs);
        let mut sim = PassengerSim::new(cabin, &paths, book.clone(), seed);
        // Aisles carry a crowd: standing room beyond what the cabin file lists.
        sim.add_standing_room();
        sim
    }

    /// Adds the unit behind the chain (its cabin joins the others'); nobody may be aboard yet.
    pub fn add_unit(&mut self, root: Entity, unit: Unit, store: Entity) {
        self.units.push(UnitSource { root, unit, store });
        self.sim = Self::sim_of(&self.units, &self.book, self.seed);
    }
}
