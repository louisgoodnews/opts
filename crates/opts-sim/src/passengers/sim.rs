//! The passenger simulation of one unit.

use super::destination::Destination;
use super::graph::CabinGraph;
use super::rng::Rng;
use super::spec::{Cabin, Paths, TicketBook};
use super::state::Next;
pub use super::state::{Doors, Event, Passenger, Phase};
use glam::Vec3;

mod step;

/// Walking speed inside the vehicle (m/s).
pub const WALK_SPEED_M_S: f32 = 1.1;
/// Seconds at the ticket machine to buy.
const BUY_S: f32 = 4.0;
/// Seconds at the stamper.
const STAMP_S: f32 = 1.5;
/// Chance that a passenger takes a free seat rather than standing.
const SIT_CHANCE: f32 = 0.7;
/// Walking distance (m) over which a free place becomes half as likely to be chosen.
pub const PLACE_SPREAD_M: f32 = 6.0;

/// The passengers of one unit.
#[derive(Debug, Clone)]
pub struct PassengerSim {
    /// The cabin layout.
    pub cabin: Cabin,
    /// The ticket system of the map.
    pub tickets: TicketBook,
    pub(super) graph: CabinGraph,
    pub(super) place_node: Vec<usize>,
    pub(super) entry_nodes: Vec<usize>,
    pub(super) exit_nodes: Vec<usize>,
    pub(super) passengers: Vec<Passenger>,
    pub(super) events: Vec<Event>,
    pub(super) rng: Rng,
    next_id: u32,
    /// Money taken for tickets so far.
    pub revenue: f32,
}

impl PassengerSim {
    /// A simulation for `cabin` on `paths` with `tickets`, random seed `seed`.
    #[must_use]
    pub fn new(cabin: Cabin, paths: &Paths, tickets: TicketBook, seed: u64) -> Self {
        let graph = CabinGraph::new(paths);
        let reach = graph.reachable(&cabin.entries);
        let place_node = cabin
            .places
            .iter()
            .map(|p| {
                graph
                    .nearest_reachable(p.position, p.unit, &reach)
                    .unwrap_or(0)
            })
            .collect();
        let entry_nodes = cabin.entries.clone();
        let exit_nodes = cabin.exits.clone();
        Self {
            cabin,
            tickets,
            graph,
            place_node,
            entry_nodes,
            exit_nodes,
            passengers: Vec::new(),
            events: Vec::new(),
            rng: Rng::new(seed),
            next_id: 1,
            revenue: 0.0,
        }
    }

    /// Adds the standing room of [`standing_room`](super::standing_room) to the cabin: the
    /// places after the listed ones. Returns how many were added.
    pub fn add_standing_room(&mut self) -> usize {
        let extra = super::crowd::standing_room(&self.cabin, &self.graph);
        let n = extra.len();
        let reach = self.graph.reachable(&self.entry_nodes);
        for p in extra {
            let node = self
                .graph
                .nearest_reachable(p.position, p.unit, &reach)
                .unwrap_or(0);
            self.place_node.push(node);
            self.cabin.places.push(p);
        }
        n
    }

    /// How many people the cabin holds: every place, seats and standing.
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.cabin.places.len()
    }

    /// The path graph.
    #[must_use]
    pub fn graph(&self) -> &CabinGraph {
        &self.graph
    }

    /// The passengers.
    #[must_use]
    pub fn passengers(&self) -> &[Passenger] {
        &self.passengers
    }

    /// Removes everybody (passengers switched off).
    pub fn clear(&mut self) {
        self.passengers.clear();
        self.events.clear();
    }

    /// Takes the events since the last call.
    pub fn take_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }
}

mod board;
mod requests;
mod tickets;
