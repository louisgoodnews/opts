//! Getting off at the doors of the unit behind the joint, and paths with a gap in them.

use super::*;
use glam::Vec3;

const DT: f32 = 0.05;

fn pack() -> TicketBook {
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
fn head() -> Unit {
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
fn rear(gap: f32) -> Unit {
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

fn distance_to_segment(p: Vec3, a: Vec3, b: Vec3) -> f32 {
    let ab = b - a;
    let t = if ab.length_squared() > 0.0 {
        ((p - a).dot(ab) / ab.length_squared()).clamp(0.0, 1.0)
    } else {
        0.0
    };
    p.distance(a + ab * t)
}

/// Whether a passenger in `unit` at `p` stands on a link (or a bridge) of the paths, or on the
/// short way between a place and the pathpoint it is reached from.
fn on_the_paths(s: &PassengerSim, unit: usize, p: Vec3) -> bool {
    let g = s.graph();
    let reach = g.reachable(&s.cabin.entries);
    let on_link = g
        .links
        .iter()
        .chain(&g.bridges)
        .filter(|&&(a, _)| g.unit(a) == unit)
        .any(|&(a, b)| distance_to_segment(p, g.points[a], g.points[b]) < 0.02);
    let on_stub = s
        .cabin
        .places
        .iter()
        .filter(|pl| pl.unit == unit)
        .any(|pl| {
            g.nearest_reachable(pl.position, unit, &reach)
                .is_some_and(|n| distance_to_segment(p, pl.position, g.points[n]) < 0.02)
        });
    on_link || on_stub
}

/// Runs `seconds` with `doors`, checking that nobody leaves the paths or jumps; returns the exits
/// people got off through.
fn run_checked(s: &mut PassengerSim, seconds: f32, doors: &Doors) -> Vec<usize> {
    let mut out = Vec::new();
    let mut last: Vec<(u32, usize, Vec3)> = Vec::new();
    for _ in 0..(seconds / DT) as usize {
        s.step(DT, doors);
        for p in s.passengers() {
            if matches!(p.phase, Phase::WaitingOutside { .. }) {
                continue;
            }
            assert!(
                on_the_paths(s, p.unit, p.position),
                "passenger {} left the paths at {:?} (unit {})",
                p.id,
                p.position,
                p.unit
            );
            if let Some(&(_, u, q)) = last.iter().find(|l| l.0 == p.id) {
                // A step at walking speed; only sitting down puts the person onto the seat.
                if u == p.unit && !matches!(p.phase, Phase::Settled { .. }) {
                    assert!(
                        q.distance(p.position) <= WALK_SPEED_M_S * DT + 1e-3,
                        "passenger {} jumped from {q:?} to {:?}",
                        p.id,
                        p.position
                    );
                }
            }
        }
        last = s
            .passengers()
            .iter()
            .map(|p| (p.id, p.unit, p.position))
            .collect();
        for e in s.take_events() {
            if let Event::Alighted { exit, .. } = e {
                out.push(exit);
            }
        }
    }
    out
}

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
