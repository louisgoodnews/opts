//! The passenger simulation of one unit.

use super::graph::CabinGraph;
use super::spec::{Cabin, Paths, TicketBook};
use glam::Vec3;

/// Walking speed inside the vehicle (m/s).
pub const WALK_SPEED_M_S: f32 = 1.1;
/// Seconds at the ticket machine to buy.
const BUY_S: f32 = 4.0;
/// Seconds at the stamper.
const STAMP_S: f32 = 1.5;
/// Chance that a passenger takes a free seat rather than standing.
const SIT_CHANCE: f32 = 0.7;

/// A small deterministic random source (SplitMix64).
#[derive(Debug, Clone)]
pub struct Rng(u64);

impl Rng {
    /// A source with `seed`.
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    /// The next value in `[0, 1)`.
    pub fn next_f32(&mut self) -> f32 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut x = self.0;
        x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        x ^= x >> 31;
        (x >> 40) as f32 / 16_777_216.0
    }
}

/// What a passenger does now.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Phase {
    /// Outside, waiting for entry door `entry` to open.
    WaitingOutside {
        /// Index into the cabin's entries.
        entry: usize,
    },
    /// Walking to a pathpoint.
    Walking {
        /// The goal pathpoint.
        to: usize,
    },
    /// Buying a ticket at a sale point.
    Buying {
        /// Seconds left.
        left_s: f32,
    },
    /// Stamping at a validator.
    Stamping {
        /// Seconds left.
        left_s: f32,
    },
    /// Sitting or standing on a place.
    Settled {
        /// Index into the cabin's places.
        place: usize,
    },
    /// At exit `exit`, waiting for its door to open.
    WaitingToLeave {
        /// Index into the cabin's exits.
        exit: usize,
    },
}

/// What the passenger does after the current phase.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Next {
    Sale,
    Stamp,
    Place,
    Exit,
}

/// A passenger.
#[derive(Debug, Clone, PartialEq)]
pub struct Passenger {
    /// Identifier, unique in the simulation.
    pub id: u32,
    /// Age in years (tickets are bought by age).
    pub age: u32,
    /// Where in the plan the passenger is.
    pub phase: Phase,
    /// The ticket held (index in the ticket pack).
    pub ticket: Option<usize>,
    /// The ticket was stamped.
    pub stamped: bool,
    /// Stops to ride before getting off.
    pub stops_left: u32,
    /// Position in the space of the unit the passenger is in.
    pub position: Vec3,
    /// The unit of the chain the passenger is in.
    pub unit: usize,
    /// The pathpoint last reached.
    pub at: usize,
    route: Vec<usize>,
    next: Next,
    place: Option<usize>,
}

/// The state of the doors the passengers use, by entry and exit index of the cabin.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Doors {
    /// Entry doors open.
    pub entries: Vec<bool>,
    /// Exit doors open.
    pub exits: Vec<bool>,
}

/// Something that happened (for the sounds, the scripts and the displays).
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// A passenger came in through entry `entry`.
    Boarded {
        /// Passenger.
        id: u32,
        /// Index into the cabin's entries.
        entry: usize,
    },
    /// A ticket was bought.
    Sold {
        /// Passenger.
        id: u32,
        /// Index in the ticket pack.
        ticket: usize,
        /// Price.
        price: f32,
    },
    /// A ticket was stamped.
    Stamped {
        /// Passenger.
        id: u32,
    },
    /// A passenger took a seat or a standing place.
    Settled {
        /// Passenger.
        id: u32,
        /// Index into the cabin's places.
        place: usize,
        /// A seat (else standing).
        seated: bool,
    },
    /// A passenger got off through exit `exit`.
    Alighted {
        /// Passenger.
        id: u32,
        /// Index into the cabin's exits.
        exit: usize,
    },
}

/// The passengers of one unit.
#[derive(Debug, Clone)]
pub struct PassengerSim {
    /// The cabin layout.
    pub cabin: Cabin,
    /// The ticket system of the map.
    pub tickets: TicketBook,
    graph: CabinGraph,
    place_node: Vec<usize>,
    entry_nodes: Vec<usize>,
    exit_nodes: Vec<usize>,
    passengers: Vec<Passenger>,
    events: Vec<Event>,
    rng: Rng,
    next_id: u32,
    /// Money taken for tickets so far.
    pub revenue: f32,
}

impl PassengerSim {
    /// A simulation for `cabin` on `paths` with `tickets`, random seed `seed`.
    #[must_use]
    pub fn new(cabin: Cabin, paths: &Paths, tickets: TicketBook, seed: u64) -> Self {
        let graph = CabinGraph::new(paths);
        let place_node = cabin
            .places
            .iter()
            .map(|p| graph.nearest_in_unit(p.position, p.unit).unwrap_or(0))
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
    fn draw_ticket(&mut self, age: u32, stops: u32) -> Option<usize> {
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
    /// one, else a standing place; the nearest by walking distance.
    fn pick_place(&mut self, from: usize) -> Option<usize> {
        let taken: Vec<usize> = self.passengers.iter().filter_map(|p| p.place).collect();
        let free = |i: &usize| !taken.contains(i);
        let nearest = |this: &Self, seats: bool| {
            (0..this.cabin.places.len())
                .filter(|i| free(i) && this.cabin.places[*i].is_seat() == seats)
                .filter_map(|i| {
                    this.graph
                        .route(from, this.place_node[i])
                        .map(|(_, len)| (i, len))
                })
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .map(|(i, _)| i)
        };
        let want_seat = self.rng.next_f32() < SIT_CHANCE;
        let order = if want_seat {
            [true, false]
        } else {
            [false, true]
        };
        order.iter().find_map(|&seats| nearest(self, seats))
    }

    /// Advances the passengers by `dt` seconds with the doors as given.
    pub fn step(&mut self, dt: f32, doors: &Doors) {
        let mut gone = Vec::new();
        for i in 0..self.passengers.len() {
            match self.passengers[i].phase {
                Phase::WaitingOutside { entry } => {
                    if doors.entries.get(entry).copied().unwrap_or(false) {
                        self.enter(i, entry);
                    }
                }
                Phase::Walking { to } => self.walk(i, to, dt),
                Phase::Buying { left_s } => {
                    if left_s - dt <= 0.0 {
                        self.finish_buying(i);
                    } else {
                        self.passengers[i].phase = Phase::Buying {
                            left_s: left_s - dt,
                        };
                    }
                }
                Phase::Stamping { left_s } => {
                    if left_s - dt <= 0.0 {
                        let p = &mut self.passengers[i];
                        p.stamped = true;
                        let id = p.id;
                        self.events.push(Event::Stamped { id });
                        self.go_to_place(i);
                    } else {
                        self.passengers[i].phase = Phase::Stamping {
                            left_s: left_s - dt,
                        };
                    }
                }
                Phase::Settled { .. } => {}
                Phase::WaitingToLeave { exit } => {
                    if doors.exits.get(exit).copied().unwrap_or(false) {
                        gone.push((i, exit));
                    }
                }
            }
        }
        for &(i, exit) in gone.iter().rev() {
            let p = self.passengers.remove(i);
            self.events.push(Event::Alighted { id: p.id, exit });
        }
    }

    /// The passenger came through the door: decides about the ticket.
    fn enter(&mut self, i: usize, entry: usize) {
        let (id, age, stops) = {
            let p = &self.passengers[i];
            (p.id, p.age, p.stops_left)
        };
        self.events.push(Event::Boarded { id, entry });
        let b = self.tickets.behaviour;
        let entry_sells = !self
            .cabin
            .entry_no_sale
            .get(entry)
            .copied()
            .unwrap_or(false);
        let buys = !self.cabin.no_ticket_sale
            && entry_sells
            && !self.cabin.ticket_sales.is_empty()
            && self.rng.next_f32() < b.ticketbuy_prop;
        let ticket = if buys {
            self.draw_ticket(age, stops)
        } else {
            None
        };
        if let Some(t) = ticket {
            let node = self.cabin.ticket_sales[0];
            let p = &mut self.passengers[i];
            p.ticket = Some(t);
            p.next = Next::Sale;
            p.phase = Phase::Walking { to: node };
            return;
        }
        let stamps = !self.cabin.stampers.is_empty() && self.rng.next_f32() < b.stamper_prop;
        if stamps {
            let node = self.cabin.stampers[0];
            let p = &mut self.passengers[i];
            p.next = Next::Stamp;
            p.phase = Phase::Walking { to: node };
        } else {
            self.go_to_place(i);
        }
    }

    fn finish_buying(&mut self, i: usize) {
        let (id, t) = {
            let p = &self.passengers[i];
            (p.id, p.ticket)
        };
        if let Some(t) = t {
            let price = self.tickets.price(t).unwrap_or(0.0);
            self.revenue += price;
            self.events.push(Event::Sold {
                id,
                ticket: t,
                price,
            });
            // A bought ticket is validated by the machine itself.
            self.passengers[i].stamped = true;
        }
        self.go_to_place(i);
    }

    fn go_to_place(&mut self, i: usize) {
        let from = self.passengers[i].at;
        match self.pick_place(from) {
            Some(place) => {
                let node = self.place_node[place];
                let p = &mut self.passengers[i];
                p.place = Some(place);
                p.next = Next::Place;
                p.route.clear();
                p.phase = Phase::Walking { to: node };
            }
            None => {
                // The cabin is full: the passenger stays in the aisle where they are.
                let p = &mut self.passengers[i];
                p.next = Next::Place;
                p.phase = Phase::Settled { place: usize::MAX };
            }
        }
    }

    fn walk(&mut self, i: usize, to: usize, dt: f32) {
        if self.passengers[i].route.is_empty() && self.passengers[i].at != to {
            match self.graph.route(self.passengers[i].at, to) {
                Some((r, _)) => self.passengers[i].route = r,
                None => {
                    // No way: the passenger arrives where they are.
                    self.passengers[i].at = to;
                }
            }
        }
        let mut budget = WALK_SPEED_M_S * dt;
        while budget > 0.0 {
            let p = &mut self.passengers[i];
            let Some(&next) = p.route.first() else {
                break;
            };
            let target = self.graph.points[next];
            // The joint between two units is a hop: the two points are in different spaces and in
            // the same place of the vehicle.
            if self.graph.unit(next) != p.unit {
                p.position = target;
                p.unit = self.graph.unit(next);
                p.at = next;
                p.route.remove(0);
                continue;
            }
            let d = p.position.distance(target);
            if d <= budget {
                budget -= d;
                p.position = target;
                p.at = next;
                p.route.remove(0);
            } else {
                p.position += (target - p.position) / d * budget;
                budget = 0.0;
            }
        }
        if self.passengers[i].route.is_empty() && self.passengers[i].at == to {
            self.arrived(i);
        }
    }

    fn arrived(&mut self, i: usize) {
        match self.passengers[i].next {
            Next::Sale => {
                self.passengers[i].phase = Phase::Buying { left_s: BUY_S };
            }
            Next::Stamp => {
                self.passengers[i].phase = Phase::Stamping { left_s: STAMP_S };
            }
            Next::Place => {
                let id = self.passengers[i].id;
                let place = self.passengers[i].place.unwrap_or(usize::MAX);
                if let Some(pl) = self.cabin.places.get(place) {
                    self.passengers[i].position = pl.position;
                    self.passengers[i].unit = pl.unit;
                    self.events.push(Event::Settled {
                        id,
                        place,
                        seated: pl.is_seat(),
                    });
                }
                self.passengers[i].phase = Phase::Settled { place };
            }
            Next::Exit => {
                let at = self.passengers[i].at;
                let exit = self.exit_nodes.iter().position(|&n| n == at).unwrap_or(0);
                self.passengers[i].phase = Phase::WaitingToLeave { exit };
            }
        }
    }
}
