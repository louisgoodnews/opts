//! People behind the joint, and places beyond a cut in the paths.

use super::*;

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
