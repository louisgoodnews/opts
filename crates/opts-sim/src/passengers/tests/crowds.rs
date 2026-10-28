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

#[test]
fn standing_room_fills_the_aisle_but_not_the_seats_or_the_doors() {
    let (cabin, paths) = aisle_bus();
    let graph = CabinGraph::new(&paths);
    let room = standing_room(&cabin, &graph);
    assert!(
        room.len() >= 14,
        "an 8 m aisle holds a crowd: {}",
        room.len()
    );
    assert!(room.iter().all(|p| !p.is_seat()));
    for p in &room {
        for s in &cabin.places {
            let d = Vec3::new(
                p.position.x - s.position.x,
                p.position.y - s.position.y,
                0.0,
            )
            .length();
            assert!(d >= 0.38 - 1e-4, "clear of the seats: {d}");
        }
        for &door in cabin.entries.iter().chain(&cabin.exits) {
            let q = graph.points[door];
            let d = Vec3::new(p.position.x - q.x, p.position.y - q.y, 0.0).length();
            assert!(d >= 0.8 - 1e-4, "clear of the doors: {d}");
        }
    }
    for (i, a) in room.iter().enumerate() {
        for b in &room[i + 1..] {
            let d = Vec3::new(
                a.position.x - b.position.x,
                a.position.y - b.position.y,
                0.0,
            )
            .length();
            assert!(d >= 0.4 - 1e-4, "people do not stand on each other: {d}");
        }
    }
}

#[test]
fn a_bus_takes_every_seat_and_then_every_standing_place() {
    let (cabin, paths) = aisle_bus();
    let seats = cabin.seat_count();
    let mut sim = PassengerSim::new(cabin, &paths, pack(0.0, 0.0), 7);
    let added = sim.add_standing_room();
    assert!(added > 10);
    assert_eq!(sim.capacity(), seats + added);
    let doors = Doors {
        entries: vec![true],
        exits: vec![false],
    };
    for _ in 0..sim.capacity() {
        sim.board(30, 20, 0);
    }
    for _ in 0..4000 {
        sim.step(0.1, &doors);
    }
    let sitting = (0..sim.capacity())
        .filter(|&i| sim.cabin.places[i].is_seat() && sim.on_place(i) == 1)
        .count();
    let standing = (0..sim.capacity())
        .filter(|&i| !sim.cabin.places[i].is_seat() && sim.on_place(i) == 1)
        .count();
    assert_eq!(sitting, seats, "every seat is taken");
    assert_eq!(standing, added, "and every standing place");
    assert_eq!(sim.humans_inside(), sim.capacity());
}

#[test]
fn poisson_counts_follow_their_mean() {
    let mut rng = Rng::new(3);
    for mean in [0.5f32, 4.0, 20.0, 80.0, 250.0] {
        let n = 4000;
        let total: u64 = (0..n).map(|_| u64::from(rng.poisson(mean))).sum();
        let avg = total as f32 / n as f32;
        assert!(
            (avg - mean).abs() < mean.sqrt() * 0.2 + 0.1,
            "mean {mean}: {avg}"
        );
    }
    assert_eq!(rng.poisson(0.0), 0);
    assert_eq!(rng.poisson(-3.0), 0);
}

#[test]
fn people_also_take_the_places_behind_the_joint_while_the_front_has_free_seats() {
    // Six seats in the head, one in the unit behind; three people board at the front door.
    let (cabin, paths) = aisle_bus();
    let head = Unit {
        cabin,
        paths,
        link_to_prev: Some(3),
        link_to_next: None,
    };
    let (cab, graph) = chain(&[head, trailer_unit()]);
    let (mut behind, mut front) = (0, 0);
    for seed in 0..40 {
        let mut s = PassengerSim::new(cab.clone(), &graph, pack(0.0, 0.0), seed);
        for _ in 0..3 {
            s.board(30, 5, 0);
        }
        let doors = Doors {
            entries: vec![true],
            exits: vec![false, false],
        };
        run(&mut s, 60.0, &doors);
        for p in s.passengers() {
            assert!(matches!(p.phase, Phase::Settled { .. }), "{:?}", p.phase);
            if p.unit == 1 {
                behind += 1;
            } else {
                front += 1;
            }
        }
    }
    assert!(
        behind > 0,
        "somebody walks through the joint (front {front})"
    );
    assert!(
        front > behind,
        "the nearer places stay the likelier (behind {behind})"
    );
}

#[test]
fn a_place_beyond_a_cut_in_the_paths_is_reached_from_the_nearest_point_that_can_be() {
    // The rear unit's paths end at its seat point 2; point 4 near the second seat is not linked
    // (as the Valiant 7700A's rear unit lacks the link between its points 9 and 10).
    let mut rear = trailer_unit();
    rear.paths.points.push(Vec3::new(0.0, -5.0, 0.4));
    rear.cabin.places.push(Place {
        position: Vec3::new(0.4, -5.2, 0.9),
        seat_height: 0.45,
        rotation_deg: 0.0,
        unit: 0,
    });
    let head = Unit {
        cabin: Cabin {
            places: Vec::new(),
            ..cabin()
        },
        paths: paths(),
        link_to_prev: Some(5),
        link_to_next: None,
    };
    let (cab, graph) = chain(&[head, rear]);
    let mut s = PassengerSim::new(cab, &graph, pack(0.0, 0.0), 9);
    s.board(30, 5, 0);
    s.board(40, 5, 0);
    let doors = Doors {
        entries: vec![true],
        exits: vec![false, false],
    };
    run(&mut s, 60.0, &doors);
    assert_eq!(s.on_place(0) + s.on_place(1), 2, "both seats are taken");
}
