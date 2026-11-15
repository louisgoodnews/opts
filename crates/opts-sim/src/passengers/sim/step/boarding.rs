//! The passenger at the door: the ticket decision, the sale and the way to a place.

use super::super::*;

impl PassengerSim {
    /// The passenger came through the door: decides about the ticket.
    pub(super) fn enter(&mut self, i: usize, entry: usize) {
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

    pub(super) fn finish_buying(&mut self, i: usize) {
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

    pub(super) fn go_to_place(&mut self, i: usize) {
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
}
