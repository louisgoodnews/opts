//! alighting, walking the links, seeds and the sales counter.

use super::*;

#[test]
fn at_the_stop_those_who_are_done_walk_to_the_exit_and_get_off_when_it_opens() {
    let mut s = sim(0.0, 0.0, 5);
    s.board(30, 1, 0);
    s.board(30, 3, 0);
    run(&mut s, 30.0, &open());
    assert_eq!(s.humans_inside(), 2);
    s.take_events();
    s.arrive_at_stop();
    run(&mut s, 30.0, &shut());
    assert_eq!(s.humans_inside(), 2, "the exit is shut: nobody gets off");
    assert_eq!(s.exit_requests(), vec![true], "one waits at the exit door");
    run(&mut s, 2.0, &open());
    assert_eq!(s.humans_inside(), 1);
    let ev = s.take_events();
    assert!(ev
        .iter()
        .any(|e| matches!(e, Event::Alighted { exit: 0, .. })));
    // The place of the one who left is free again.
    let on: usize = (0..2).map(|i| s.on_place(i)).sum();
    assert_eq!(on, 1);
}

#[test]
fn walkers_are_counted_on_their_link() {
    let mut s = sim(0.0, 0.0, 2);
    s.board(30, 3, 0);
    run(&mut s, 0.1, &open());
    // After the door the passenger walks 0 → 1: link 0.
    run(&mut s, 0.5, &open());
    let total: usize = (0..s.graph().links.len()).map(|l| s.on_link(l)).sum();
    assert_eq!(total, 1);
    assert_eq!(s.on_link(0), 1);
}

#[test]
fn the_shortest_way_follows_the_links() {
    let g = CabinGraph::new(&paths());
    let (route, len) = g.route(0, 4).expect("route");
    assert_eq!(route, vec![1, 4]);
    let want = g.points[0].distance(g.points[1]) + g.points[1].distance(g.points[4]);
    assert!((len - want).abs() < 1e-4);
    assert_eq!(g.route(3, 3), Some((vec![], 0.0)));
    assert_eq!(g.nearest(Vec3::new(0.4, 2.1, 0.4)), Some(3));
}

#[test]
fn the_same_seed_gives_the_same_day() {
    let events = |seed| {
        let mut s = sim(0.5, 0.5, seed);
        for age in [8, 30, 67, 25] {
            s.board(age, 3, 0);
        }
        run(&mut s, 80.0, &open());
        s.take_events()
    };
    assert_eq!(events(42), events(42));
}

#[test]
fn nobody_buys_who_boards_through_an_entry_that_does_not_sell() {
    let mut s = sim(1.0, 0.0, 9);
    s.cabin.entry_no_sale = vec![true];
    for _ in 0..4 {
        s.board(30, 3, 0);
    }
    run(&mut s, 60.0, &open());
    assert_eq!(s.revenue, 0.0);
    assert_eq!(s.humans_inside(), 4);
}
