//! Unit tests for passenger boarding, ticket sales, seating, alighting and cabin path routing.

use super::*;
use glam::Vec3;

mod boarding;
mod crowds;
mod journey;
mod trailer;

fn paths() -> Paths {
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

fn cabin() -> Cabin {
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

fn pack(buy: f32, stamp: f32) -> TicketBook {
    TicketBook {
        behaviour: Behaviour {
            stamper_prop: stamp,
            ticketbuy_prop: buy,
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

fn sim(buy: f32, stamp: f32, seed: u64) -> PassengerSim {
    PassengerSim::new(cabin(), &paths(), pack(buy, stamp), seed)
}

fn open() -> Doors {
    Doors {
        entries: vec![true],
        exits: vec![true],
    }
}

fn shut() -> Doors {
    Doors {
        entries: vec![false],
        exits: vec![false],
    }
}

fn run(s: &mut PassengerSim, seconds: f32, doors: &Doors) {
    for _ in 0..(seconds * 20.0) as usize {
        s.step(0.05, doors);
    }
}

/// A rear unit: a joint point 0, an aisle point 1, a seat point 2 and an exit door point 3.
fn trailer_unit() -> Unit {
    let p = |x: f32, y: f32| Vec3::new(x, y, 0.4);
    Unit {
        cabin: Cabin {
            places: vec![Place {
                position: p(0.4, -3.0),
                seat_height: 0.45,
                rotation_deg: 0.0,
                unit: 0,
            }],
            entries: Vec::new(),
            exits: vec![3],
            stampers: Vec::new(),
            ticket_sales: Vec::new(),
            no_ticket_sale: false,
            entry_no_sale: Vec::new(),
        },
        paths: Paths {
            points: vec![p(0.0, 4.0), p(0.0, 0.0), p(0.4, -3.0), p(1.2, -2.0)],
            links: vec![(0, 1), (1, 2), (1, 3)],
            one_way: Vec::new(),
            unit_of: Vec::new(),
        },
        link_to_prev: None,
        link_to_next: Some(0),
    }
}
