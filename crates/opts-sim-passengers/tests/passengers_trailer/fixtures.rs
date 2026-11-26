//! The head unit, the rear unit behind a joint and the ticket book of the trailer tests.

use opts_sim_passengers::passengers::*;
use glam::Vec3;

pub(super) const DT: f32 = 0.05;

pub(super) fn pack() -> TicketBook {
    TicketBook {
        behaviour: Behaviour {
            stamper_prop: 0.0,
            ticketbuy_prop: 0.0,
            chattiness: 1.0,
            whinge_prop: 0.0,
        },
        tickets: Vec::new(),
    }
}

/// A head unit with one door (entry and exit at point 0) and no places, its joint at point 2.
pub(super) fn head() -> Unit {
    let p = |x: f32, y: f32| Vec3::new(x, y, 0.4);
    Unit {
        cabin: Cabin {
            entries: vec![0],
            exits: vec![0],
            ..Cabin::default()
        },
        paths: Paths {
            points: vec![p(1.2, 2.0), p(0.0, 2.0), p(0.0, -3.0)],
            links: vec![(0, 1), (1, 2)],
            one_way: Vec::new(),
            unit_of: Vec::new(),
        },
        link_to_prev: Some(2),
        link_to_next: None,
    }
}

/// A rear unit built like the Valiant 7700A's: the joint (point 0), an aisle to its only door
/// at the front (exit point 2), and the rear half of the aisle (points 4 and 5) not linked to the
/// front half (point 3) although they lie `gap` m apart. Four seats on podiums in the rear half.
pub(super) fn rear(gap: f32) -> Unit {
    let p = |x: f32, y: f32| Vec3::new(x, y, 0.4);
    let seat = |x: f32, y: f32| Place {
        position: Vec3::new(x, y, 0.85),
        seat_height: 0.45,
        rotation_deg: 0.0,
        unit: 0,
    };
    Unit {
        cabin: Cabin {
            places: vec![
                seat(-0.8, -1.0),
                seat(0.8, -1.0),
                seat(-0.8, -2.5),
                seat(0.8, -2.5),
            ],
            exits: vec![2],
            ..Cabin::default()
        },
        paths: Paths {
            points: vec![
                p(0.0, 3.0),
                p(0.0, 1.5),
                p(1.2, 1.5),
                p(0.0, 0.0),
                p(0.0, -gap),
                p(0.0, -2.5),
            ],
            links: vec![(0, 1), (1, 2), (1, 3), (4, 5)],
            one_way: Vec::new(),
            unit_of: Vec::new(),
        },
        link_to_prev: None,
        link_to_next: Some(0),
    }
}
