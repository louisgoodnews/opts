//! The simulation's plain data from the parsed OMSI files.

use opts_core::formats::ticketpack::TicketPack;
use opts_core::formats::vehicles::cabin::PassengerCabin;
use opts_core::formats::vehicles::PassengerPaths;
use opts_sim::passengers::{Behaviour, Cabin, Paths, Place, Ticket, TicketBook};

/// The cabin of the simulation from the parsed `passengercabin.cfg`.
#[must_use]
pub fn cabin_spec(c: &PassengerCabin) -> Cabin {
    Cabin {
        places: c
            .places
            .iter()
            .map(|p| Place {
                position: p.position,
                seat_height: p.seat_height,
                rotation_deg: p.rotation_deg,
            })
            .collect(),
        entries: c.entries.clone(),
        exits: c.exits.clone(),
        stampers: c.stampers.iter().map(|m| m.pathpoint).collect(),
        ticket_sales: c.ticket_sales.iter().map(|m| m.pathpoint).collect(),
        no_ticket_sale: c.no_ticket_sale,
    }
}

/// The graph of the simulation from the parsed `paths.cfg`.
#[must_use]
pub fn paths_spec(p: &PassengerPaths) -> Paths {
    Paths {
        points: p.points.iter().map(|p| p.position).collect(),
        links: p.links.iter().map(|l| (l.from, l.to)).collect(),
    }
}

/// The ticket system of the simulation from a parsed `.otp`.
#[must_use]
pub fn ticket_book(t: &TicketPack) -> TicketBook {
    TicketBook {
        behaviour: Behaviour {
            stamper_prop: t.behaviour.stamper_prop,
            ticketbuy_prop: t.behaviour.ticketbuy_prop,
            chattiness: t.behaviour.chattiness,
            whinge_prop: t.behaviour.whinge_prop,
        },
        tickets: t
            .tickets
            .iter()
            .map(|k| Ticket {
                display: k.display_string.clone(),
                value: k.value,
                age_min: k.age_min,
                age_max: k.age_max,
                max_stations: k.max_stations,
                probability: k.probability,
                day_ticket: k.day_ticket,
            })
            .collect(),
    }
}
