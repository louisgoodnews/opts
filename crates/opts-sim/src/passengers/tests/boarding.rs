//! boarding through an open door, ticket sales and seating.

use super::*;

#[test]
fn a_passenger_boards_only_through_an_open_door() {
    let mut s = sim(0.0, 0.0, 1);
    s.board(30, 3, 0).expect("entry");
    run(&mut s, 5.0, &shut());
    assert_eq!(s.humans_inside(), 0, "nobody inside a closed door");
    assert_eq!(s.entry_requests(), vec![true], "the door is requested");
    run(&mut s, 1.0, &open());
    assert_eq!(s.humans_inside(), 1);
    assert_eq!(s.entry_requests(), vec![false]);
    assert!(matches!(
        s.take_events()[0],
        Event::Boarded { entry: 0, .. }
    ));
}

#[test]
fn a_ticket_is_bought_and_paid_and_then_a_place_is_taken() {
    let mut s = sim(1.0, 0.0, 7);
    s.board(30, 3, 0);
    run(&mut s, 40.0, &open());
    let ev = s.take_events();
    assert!(
        ev.iter()
            .any(|e| matches!(e, Event::Sold { price, .. } if (*price - 2.5).abs() < 1e-6)),
        "{ev:?}"
    );
    assert!((s.revenue - 2.5).abs() < 1e-6);
    let settled = ev
        .iter()
        .find_map(|e| match e {
            Event::Settled { place, seated, .. } => Some((*place, *seated)),
            _ => None,
        })
        .expect("took a place");
    assert_eq!(s.on_place(settled.0), 1);
    assert!(s.passengers()[0].stamped, "a bought ticket is valid");
}

#[test]
fn nobody_buys_when_the_pack_says_so() {
    let mut s = sim(0.0, 0.0, 3);
    for _ in 0..5 {
        s.board(40, 2, 0);
    }
    run(&mut s, 60.0, &open());
    assert_eq!(s.revenue, 0.0);
    assert!(!s
        .take_events()
        .iter()
        .any(|e| matches!(e, Event::Sold { .. })));
}

#[test]
fn a_full_cabin_leaves_the_rest_standing_in_the_aisle() {
    let mut s = sim(0.0, 0.0, 11);
    for _ in 0..4 {
        s.board(30, 5, 0);
    }
    run(&mut s, 60.0, &open());
    assert_eq!(s.humans_inside(), 4);
    let on_places: usize = (0..2).map(|i| s.on_place(i)).sum();
    assert_eq!(on_places, 2, "two places, two people on them");
    assert!(
        s.passengers()
            .iter()
            .filter(|p| p.phase == Phase::Settled { place: usize::MAX })
            .count()
            == 2
    );
}
