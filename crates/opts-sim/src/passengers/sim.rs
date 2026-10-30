//! The passenger simulation of one unit.

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

    /// A passenger of `age` who rides `stops` stops arrives at entry `entry` (clamped to the
    /// entries there are). `None` when the unit has no entry.
    pub fn board(&mut self, age: u32, stops: u32, entry: usize) -> Option<u32> {
        if self.entry_nodes.is_empty() {
            return None;
        }
        let entry = entry.min(self.entry_nodes.len() - 1);
        let node = self.entry_nodes[entry];
        let id = self.next_id;
        self.next_id += 1;
        self.passengers.push(Passenger {
            id,
            age,
            phase: Phase::WaitingOutside { entry },
            ticket: None,
            stamped: false,
            stops_left: stops.max(1),
            position: self.graph.points.get(node).copied().unwrap_or(Vec3::ZERO),
            unit: self.graph.unit(node),
            at: node,
            route: Vec::new(),
            next: Next::Place,
            place: None,
        });
        Some(id)
    }

    /// Passengers inside the unit (not waiting outside).
    #[must_use]
    pub fn humans_inside(&self) -> usize {
        self.passengers
            .iter()
            .filter(|p| !matches!(p.phase, Phase::WaitingOutside { .. }))
            .count()
    }

    /// Passengers inside each of `units` units of the chain, head first (M-0124): the people
    /// aboard counted by the unit they are in; a unit beyond the list holds nobody.
    #[must_use]
    pub fn humans_inside_by_unit(&self, units: usize) -> Vec<usize> {
        let mut counts = vec![0; units];
        for p in &self.passengers {
            if !matches!(p.phase, Phase::WaitingOutside { .. }) {
                if let Some(c) = counts.get_mut(p.unit) {
                    *c += 1;
                }
            }
        }
        counts
    }

    /// Passengers on place `place` (`GetHumanCountOnSeat`).
    #[must_use]
    pub fn on_place(&self, place: usize) -> usize {
        self.passengers
            .iter()
            .filter(|p| p.phase == Phase::Settled { place })
            .count()
    }

    /// Passengers walking along link `link` (`GetHumanCountOnPathLink`).
    #[must_use]
    pub fn on_link(&self, link: usize) -> usize {
        self.passengers
            .iter()
            .filter(|p| {
                matches!(p.phase, Phase::Walking { .. })
                    && p.route
                        .first()
                        .is_some_and(|&to| self.graph.link_between(p.at, to) == Some(link))
            })
            .count()
    }

    /// Entry doors somebody waits at (`PAX_Entry<n>_Req`).
    #[must_use]
    pub fn entry_requests(&self) -> Vec<bool> {
        let mut r = vec![false; self.entry_nodes.len()];
        for p in &self.passengers {
            if let Phase::WaitingOutside { entry } = p.phase {
                r[entry] = true;
            }
        }
        r
    }

    /// Exit doors somebody waits at (`PAX_Exit<n>_Req`).
    #[must_use]
    pub fn exit_requests(&self) -> Vec<bool> {
        let mut r = vec![false; self.exit_nodes.len()];
        for p in &self.passengers {
            if let Phase::WaitingToLeave { exit } = p.phase {
                r[exit] = true;
            }
        }
        r
    }

    /// The bus reached a stop: every passenger whose ride is over heads for the nearest exit.
    pub fn arrive_at_stop(&mut self) {
        for i in 0..self.passengers.len() {
            let p = &mut self.passengers[i];
            if !matches!(
                p.phase,
                Phase::Settled { .. }
                    | Phase::Walking { .. }
                    | Phase::Buying { .. }
                    | Phase::Stamping { .. }
            ) {
                continue;
            }
            p.stops_left = p.stops_left.saturating_sub(1);
            if p.stops_left == 0 {
                p.place = None;
                p.next = Next::Exit;
                let from = p.at;
                let exit = self.nearest_exit(from);
                let p = &mut self.passengers[i];
                p.route.clear();
                p.phase = Phase::Walking {
                    to: self.exit_nodes.get(exit).copied().unwrap_or(from),
                };
            }
        }
    }

    fn nearest_exit(&self, from: usize) -> usize {
        (0..self.exit_nodes.len())
            .filter_map(|e| {
                self.graph
                    .route(from, self.exit_nodes[e])
                    .map(|(_, len)| (e, len))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map_or(0, |(e, _)| e)
    }

    /// The tickets a passenger of `age` may buy for `stops` stops, one drawn by weight.
    pub(super) fn draw_ticket(&mut self, age: u32, stops: u32) -> Option<usize> {
        let eligible: Vec<(usize, f32)> = self
            .tickets
            .eligible(age, stops)
            .map(|(i, t)| (i, t.probability.max(0.0)))
            .collect();
        let total: f32 = eligible.iter().map(|e| e.1).sum();
        if eligible.is_empty() || total <= 0.0 {
            return None;
        }
        let mut x = self.rng.next_f32() * total;
        for (i, w) in &eligible {
            if x < *w {
                return Some(*i);
            }
            x -= w;
        }
        eligible.last().map(|e| e.0)
    }

    /// A free place for a passenger at `from`: a seat with the given chance and when there is
    /// one, else a standing place; drawn among the free ones that can be walked to, a place
    /// [`PLACE_SPREAD_M`] farther away half as likely, so that people fill the whole vehicle (the
    /// unit behind the joint too) and not only the places next to the door.
    pub(super) fn pick_place(&mut self, from: usize) -> Option<usize> {
        let taken: Vec<usize> = self.passengers.iter().filter_map(|p| p.place).collect();
        let dist = self.graph.distances(from);
        let want_seat = self.rng.next_f32() < SIT_CHANCE;
        let roll = self.rng.next_f32();
        [want_seat, !want_seat].into_iter().find_map(|seats| {
            let free: Vec<(usize, f32)> = (0..self.cabin.places.len())
                .filter(|i| !taken.contains(i) && self.cabin.places[*i].is_seat() == seats)
                .filter_map(|i| {
                    let d = dist.get(self.place_node[i]).copied()?;
                    d.is_finite().then(|| (i, 1.0 / (1.0 + d / PLACE_SPREAD_M)))
                })
                .collect();
            let mut x = roll * free.iter().map(|f| f.1).sum::<f32>();
            free.iter()
                .find(|(_, w)| {
                    x -= w;
                    x < 0.0
                })
                .or(free.last())
                .map(|f| f.0)
        })
    }
}
