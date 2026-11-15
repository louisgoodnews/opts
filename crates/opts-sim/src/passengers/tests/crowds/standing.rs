//! Standing room, filling every seat and the Poisson arrival counts.

use super::*;

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
