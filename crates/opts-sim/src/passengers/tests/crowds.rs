//! Tests for standing room, crowds, Poisson arrival counts and articulated joint traversal.

use super::*;

fn aisle_bus() -> (Cabin, Paths) {
    // An 8 m aisle along y with a seat row on one side, doors at both ends.
    let p = |x: f32, y: f32| Vec3::new(x, y, 0.4);
    let paths = Paths {
        points: vec![
            p(1.2, 0.0),
            p(0.0, 1.5),
            p(0.0, 6.5),
            p(1.2, 8.0),
            p(0.9, 4.0),
        ],
        links: vec![(0, 1), (1, 2), (2, 3), (2, 4)],
        one_way: Vec::new(),
        unit_of: Vec::new(),
    };
    let seats = (0..6).map(|i| Place {
        position: Vec3::new(0.95, 2.5 + i as f32 * 0.75, 0.4),
        seat_height: 0.45,
        rotation_deg: 0.0,
        unit: 0,
    });
    let cabin = Cabin {
        places: seats.collect(),
        entries: vec![0],
        exits: vec![3],
        stampers: Vec::new(),
        ticket_sales: Vec::new(),
        no_ticket_sale: false,
        entry_no_sale: Vec::new(),
    };
    (cabin, paths)
}

mod joint;
mod standing;
