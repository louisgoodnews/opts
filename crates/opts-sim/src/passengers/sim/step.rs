//! Stepping passenger movement, ticket buying, stamping, and seat/standing placement.

use super::*;

impl PassengerSim {
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
        if self.passengers[i].route.is_empty() {
            let (at, unit, position) = {
                let p = &self.passengers[i];
                (p.at, p.unit, p.position)
            };
            let way = if at == to {
                Some(Vec::new())
            } else {
                self.graph.route(at, to).map(|(r, _)| r)
            };
            match way {
                Some(mut r) => {
                    // From a place (or a walk cut short) back onto the paths first: straight to
                    // the next point of the way would cut through the seats.
                    let off_path = self.graph.unit(at) == unit
                        && self
                            .graph
                            .points
                            .get(at)
                            .is_some_and(|q| q.distance(position) > 0.01);
                    if off_path {
                        r.insert(0, at);
                    }
                    self.passengers[i].route = r;
                }
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
