use super::*;
use glam::Vec3;

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
    }
}

fn cabin() -> Cabin {
    Cabin {
        places: vec![
            Place {
                position: Vec3::new(0.5, 2.0, 0.4),
                seat_height: 0.45,
                rotation_deg: 0.0,
            },
            Place {
                position: Vec3::new(-0.5, 1.5, 0.4),
                seat_height: 0.0,
                rotation_deg: 0.0,
            },
        ],
        entries: vec![0],
        exits: vec![5],
        stampers: Vec::new(),
        ticket_sales: vec![2],
        no_ticket_sale: false,
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
