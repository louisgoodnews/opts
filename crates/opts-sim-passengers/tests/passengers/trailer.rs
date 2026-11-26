//! passengers walking through the joint into the unit behind, and one-way links.

use super::*;
use opts_sim_passenger_model::graph;

#[test]
fn passengers_walk_through_the_joint_into_the_unit_behind_and_get_off_there() {
    let head = Unit {
        cabin: {
            let mut c = cabin();
            // Only the rear unit has a free seat: the head's places are taken.
            c.places.clear();
            c
        },
        paths: paths(),
        link_to_prev: Some(5),
        link_to_next: None,
    };
    let (cab, graph) = chain(&[head, trailer_unit()]);
    assert_eq!(cab.places.len(), 1);
    assert_eq!(cab.places[0].unit, 1);
    assert_eq!(cab.exits, vec![5, 6 + 3], "exits run on through the units");
    let mut s = PassengerSim::new(cab, &graph, pack(0.0, 0.0), 4);
    s.board(30, 2, 0);
    let doors = Doors {
        entries: vec![true],
        exits: vec![true, true],
    };
    run(&mut s, 40.0, &doors);
    let p = &s.passengers()[0];
    assert_eq!(p.unit, 1, "the seat is in the unit behind");
    assert_eq!(s.on_place(0), 1);
    s.arrive_at_stop();
    s.arrive_at_stop();
    run(&mut s, 40.0, &shut());
    assert_eq!(s.exit_requests().iter().filter(|r| **r).count(), 1);
    run(&mut s, 3.0, &doors);
    assert_eq!(s.humans_inside(), 0, "got off");
}

#[test]
fn a_oneway_link_can_be_walked_in_its_direction_only() {
    let mut p = paths();
    // The door leads in, the aisle does not lead back out through it.
    p.one_way = vec![true, false, false, false, false];
    let g = graph::CabinGraph::new(&p);
    assert!(g.route(0, 3).is_some());
    assert!(g.route(3, 0).is_none());
    assert!(g.route(3, 5).is_some());
}
