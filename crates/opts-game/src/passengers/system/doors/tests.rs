//! The readback of the door flags: a person boards through an entry whose `PAX_Entry0_Open` the
//! scripts set, and not through one they leave shut (ISS-013, M-0101).

use bevy::ecs::system::SystemState;
use bevy::prelude::*;
use opts_bevy::VehicleScripts;
use opts_sim::passengers::{Behaviour, Cabin, PassengerSim, Paths, Place, Ticket, TicketBook};
use opts_sim::script::ScriptEngine;

use super::read_doors;

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
        places: vec![Place {
            position: Vec3::new(0.5, 2.0, 0.4),
            seat_height: 0.45,
            rotation_deg: 0.0,
            unit: 0,
        }],
        entries: vec![0],
        exits: vec![5],
        stampers: Vec::new(),
        ticket_sales: vec![2],
        no_ticket_sale: false,
        entry_no_sale: Vec::new(),
    }
}

fn tickets() -> TicketBook {
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

/// A bus whose scripts hold `PAX_Entry0_Open` at `open` (1 or 0), with one person waiting at its
/// entry 0; returns how many are inside after ten seconds, as read through the door readback.
fn inside_after_ten_seconds(open: bool) -> usize {
    let (mut engine, _) = ScriptEngine::from_sources(&[]);
    engine.set("pax_entry0_open", if open { 1.0 } else { 0.0 });
    let mut world = World::new();
    let bus = world.spawn(VehicleScripts::new(engine)).id();
    let mut state: SystemState<Query<&VehicleScripts>> = SystemState::new(&mut world);
    let engines = state.get(&world).expect("the vehicle query");
    let doors = read_doors(&engines, &[(bus, 0)], &[]);
    let mut sim = PassengerSim::new(cabin(), &paths(), tickets(), 1);
    sim.board(30, 3, 0);
    for _ in 0..200 {
        sim.step(0.05, &doors);
    }
    sim.humans_inside()
}

#[test]
fn door_flags_decide_boarding() {
    assert_eq!(
        inside_after_ten_seconds(true),
        1,
        "a person boards through the entry the scripts open"
    );
    assert_eq!(
        inside_after_ten_seconds(false),
        0,
        "nobody boards through the entry the scripts leave shut"
    );
}
