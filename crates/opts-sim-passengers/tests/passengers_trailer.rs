//! Getting off at the doors of the unit behind the joint, and paths with a gap in them.

mod checks;
mod fixtures;

use opts_sim_passengers::passengers::*;
use checks::run_checked;
use fixtures::{head, pack, rear};
use glam::Vec3;

#[test]
fn a_short_gap_in_the_paths_is_bridged_and_a_wide_one_is_not() {
    let (_, paths) = chain(&[head(), rear(0.2)]);
    let g = CabinGraph::new(&paths);
    // The rear unit's points start after the head's three.
    assert_eq!(g.bridges, vec![(3 + 3, 3 + 4)]);
    let reach = g.reachable(&[0]);
    assert!(reach.iter().all(|r| *r), "every point can be walked to");
    let (_, paths) = chain(&[head(), rear(2.0)]);
    let g = CabinGraph::new(&paths);
    assert!(g.bridges.is_empty(), "two metres is no missing link");
}

#[test]
fn passengers_in_the_unit_behind_get_off_through_its_door_only_when_that_opens() {
    let (cabin, paths) = chain(&[head(), rear(0.2)]);
    assert_eq!(
        cabin.exits,
        vec![0, 3 + 2],
        "the head's exit, then the rear's"
    );
    let mut s = PassengerSim::new(cabin, &paths, pack(), 11);
    for _ in 0..4 {
        s.board(30, 1, 0);
    }
    let boarding = Doors {
        entries: vec![true],
        exits: vec![false, false],
    };
    assert!(run_checked(&mut s, 40.0, &boarding).is_empty());
    for place in 0..4 {
        assert_eq!(s.on_place(place), 1, "seat {place} beyond the gap is taken");
    }
    assert!(s.passengers().iter().all(|p| p.unit == 1));

    s.arrive_at_stop();
    // Only the head's door opens: everybody waits at the rear door.
    let front = Doors {
        entries: vec![false],
        exits: vec![true, false],
    };
    assert!(run_checked(&mut s, 30.0, &front).is_empty());
    assert_eq!(s.humans_inside(), 4, "nobody leaves through the wrong door");
    assert_eq!(s.exit_requests(), vec![false, true]);
    assert!(s
        .passengers()
        .iter()
        .all(|p| p.phase == Phase::WaitingToLeave { exit: 1 }
            && p.position.distance(Vec3::new(1.2, 1.5, 0.4)) < 1e-4));

    let rear_door = Doors {
        entries: vec![false],
        exits: vec![false, true],
    };
    let out = run_checked(&mut s, 5.0, &rear_door);
    assert_eq!(
        out,
        vec![1; 4],
        "all four got off through the rear unit's door"
    );
    assert_eq!(s.humans_inside(), 0);
}

#[test]
fn leaving_a_seat_goes_back_to_the_aisle_first() {
    let (cabin, paths) = chain(&[head(), rear(0.2)]);
    let mut s = PassengerSim::new(cabin, &paths, pack(), 3);
    s.board(30, 1, 0);
    let doors = Doors {
        entries: vec![true],
        exits: vec![false, false],
    };
    run_checked(&mut s, 40.0, &doors);
    s.arrive_at_stop();
    // `run_checked` fails if the way from the seat cuts across the other seats to the next point.
    run_checked(&mut s, 30.0, &doors);
    assert!(matches!(
        s.passengers()[0].phase,
        Phase::WaitingToLeave { exit: 1 }
    ));
}
