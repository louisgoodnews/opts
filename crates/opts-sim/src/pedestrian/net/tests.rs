//! Unit tests for `super` (`net.rs`).

use super::*;

fn lane(a: Vec3, b: Vec3, travel: Travel) -> FootLane {
    FootLane::new(vec![a, b], travel, 1.0).unwrap()
}

fn net() -> PedestrianNet {
    let mut n = PedestrianNet::default();
    // Two bidirectional lanes meeting at (0, 10), a one-way dead end after (0, 20).
    n.insert_tile(
        (0, 0),
        vec![
            lane(Vec3::ZERO, Vec3::new(0.0, 10.0, 0.0), Travel::Both),
            lane(
                Vec3::new(0.0, 10.0, 0.2),
                Vec3::new(0.0, 20.0, 0.2),
                Travel::Both,
            ),
            lane(
                Vec3::new(0.0, 20.0, 0.2),
                Vec3::new(0.0, 30.0, 0.2),
                Travel::Forward,
            ),
        ],
    );
    n
}

#[test]
fn continuations_respect_geometry_and_direction() {
    let n = net();
    assert_eq!(n.continuations(0, End::End), vec![(1, End::Start)]);
    assert_eq!(n.continuations(1, End::Start), vec![(0, End::End)]);
    // Lane 2 is forward-only: entering at its end is not allowed.
    assert_eq!(n.continuations(1, End::End), vec![(2, End::Start)]);
    assert!(n.continuations(2, End::End).is_empty());
}

#[test]
fn walker_crosses_lanes_turns_round_and_finishes_at_a_one_way_end() {
    let n = net();
    let mut rng = 7;
    let mut w = Walker {
        lane: 0,
        s: 5.0,
        dir: 1,
        speed: 1.0,
        phase: 0.0,
    };
    assert_eq!(w.advance(8.0, &n, &mut rng), Advance::Moving);
    assert_eq!(w.lane, 1);
    assert!((w.s - 3.0).abs() < 1e-4);
    // Run to the forward-only lane and past its end: a one-way lane cannot be turned on.
    assert_eq!(w.advance(30.0, &n, &mut rng), Advance::Finished);
}

#[test]
fn dead_end_of_a_two_way_lane_turns_the_walker_round() {
    let mut n = PedestrianNet::default();
    n.insert_tile(
        (0, 0),
        vec![lane(Vec3::ZERO, Vec3::new(0.0, 10.0, 0.0), Travel::Both)],
    );
    let mut rng = 1;
    let mut w = Walker {
        lane: 0,
        s: 9.0,
        dir: 1,
        speed: 1.0,
        phase: 0.0,
    };
    assert_eq!(w.advance(3.0, &n, &mut rng), Advance::Moving);
    assert_eq!(w.dir, -1);
    assert!((w.s - 8.0).abs() < 1e-4);
}

#[test]
fn nearest_finds_the_closest_lane_point() {
    let n = net();
    let (lane, s, d) = n.nearest(Vec3::new(2.0, 4.0, 0.0), 10.0).unwrap();
    assert_eq!(lane, 0);
    assert!((s - 4.0).abs() < 1e-4 && (d - 2.0).abs() < 1e-4);
    assert!(n.nearest(Vec3::new(50.0, 4.0, 0.0), 10.0).is_none());
}

#[test]
fn removing_a_tile_removes_its_lanes_and_links() {
    let mut n = net();
    n.remove_tile((0, 0));
    assert_eq!(n.lane_count(), 0);
    assert!(n.continuations(0, End::End).is_empty());
    let mut w = Walker {
        lane: 0,
        s: 1.0,
        dir: 1,
        speed: 1.0,
        phase: 0.0,
    };
    assert_eq!(w.advance(1.0, &n, &mut 3), Advance::Finished);
}

#[test]
fn spawn_spots_lie_in_the_requested_ring() {
    let n = net();
    let mut rng = 99;
    for _ in 0..50 {
        if let Some((id, s)) = n.random_spot((0.0, 0.0), 8.0, 22.0, &mut rng) {
            let (p, _) = n.lane(id).unwrap().at(s);
            let d = p.x.hypot(p.y);
            assert!((8.0..=22.0).contains(&d), "{d}");
        }
    }
}
