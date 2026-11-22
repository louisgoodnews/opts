//! In-code fixtures of the boarding-path tests: a bus cabin with two places and one entry, a
//! ticket book, a stop served by one trip to the Zoo (stop 1), and the doors of an open entry.

use std::collections::HashMap;
use std::sync::Arc;

use bevy::prelude::*;
use opts_sim::passengers::{
    Behaviour, Cabin, Destination, DestinationTable, Doors, PassengerSim, Paths, Place, Rng,
    Ticket, TicketBook,
};

use crate::passengers::rig::PassengerRig;

const SEED: u64 = 7;

pub(super) fn paths() -> Paths {
    let p = |x: f32, y: f32| Vec3::new(x, y, 0.4);
    Paths {
        points: vec![
            p(1.2, 4.0),  // 0 entry door
            p(0.0, 3.0),  // 1 aisle
            p(0.0, 4.5),  // 2 ticket machine
            p(0.5, 2.0),  // 3 seat
            p(-0.5, 1.5), // 4 standing place
            p(1.2, 0.0),  // 5 exit door
        ],
        links: vec![(0, 1), (1, 2), (1, 3), (1, 4), (1, 5)],
        one_way: Vec::new(),
        unit_of: Vec::new(),
    }
}

pub(super) fn cabin() -> Cabin {
    Cabin {
        places: vec![
            Place {
                position: Vec3::new(0.5, 2.0, 0.4),
                seat_height: 0.45,
                rotation_deg: 0.0,
                unit: 0,
            },
            Place {
                position: Vec3::new(-0.5, 1.5, 0.4),
                seat_height: 0.0,
                rotation_deg: 0.0,
                unit: 0,
            },
        ],
        entries: vec![0],
        exits: vec![5],
        stampers: Vec::new(),
        ticket_sales: vec![2],
        no_ticket_sale: false,
        entry_no_sale: Vec::new(),
    }
}

fn book() -> TicketBook {
    TicketBook {
        behaviour: Behaviour {
            stamper_prop: 0.0,
            ticketbuy_prop: 0.0,
            chattiness: 1.0,
            whinge_prop: 0.0,
        },
        tickets: vec![Ticket {
            display: "Single".into(),
            value: 2.5,
            age_min: 0,
            age_max: 200,
            max_stations: 0,
            probability: 1.0,
            day_ticket: false,
        }],
    }
}

pub(super) fn zoo() -> Destination {
    Destination::new("13N", "Zoo")
}

/// A rig of one bus cabin at stop `stop`. Stop 1 is served by the trip to the Zoo; no trip serves
/// any other stop, so its people have no destination.
pub(super) fn rig_at(stop: u64) -> PassengerRig {
    let mut table = DestinationTable::default();
    table.add(1, zoo(), 1);
    let book = book();
    PassengerRig {
        sim: PassengerSim::new(cabin(), &paths(), book.clone(), SEED),
        units: Vec::new(),
        book,
        seed: SEED,
        stop: None,
        stop_key: None,
        unseen: Vec::new(),
        unseen_timer: 0.0,
        actors: HashMap::new(),
        approaching: Vec::new(),
        leaving: Vec::new(),
        rng: Rng::new(SEED),
        stops_served: 0,
        force_group: 0,
        force_entry: 0,
        destinations: Some(Arc::new(table)),
        stop_id: Some(stop),
    }
}

pub(super) fn open_entry() -> Doors {
    Doors {
        entries: vec![true],
        exits: vec![false],
    }
}
