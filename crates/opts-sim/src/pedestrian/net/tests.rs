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
        lateral: 0.0,
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
        lateral: 0.0,
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
        lateral: 0.0,
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

/// A 100 m lane running north, `width` metres wide, walkable both ways.
fn north_lane(width: f32) -> PedestrianNet {
    let mut n = PedestrianNet::default();
    let lane = FootLane::new(
        vec![Vec3::ZERO, Vec3::new(0.0, 100.0, 0.0)],
        Travel::Both,
        width,
    )
    .unwrap();
    n.insert_tile((0, 0), vec![lane]);
    n
}

fn walker_on(lane: LaneId, dir: i8, lateral: f32) -> Walker {
    Walker {
        lane,
        s: 50.0,
        dir,
        speed: 1.4,
        phase: 0.0,
        lateral,
    }
}

#[test]
fn walkers_keep_to_the_right_so_opposite_walkers_pass_on_different_sides() {
    let n = north_lane(2.0);
    let (north, h_north) = walker_on(0, 1, 0.5).pose(&n).unwrap();
    let (south, h_south) = walker_on(0, -1, 0.5).pose(&n).unwrap();
    // Walking north the right hand is east; walking south it is west; half the width is 1 m.
    assert!((north.x - 0.5).abs() < 1e-5, "{north}");
    assert!((south.x + 0.5).abs() < 1e-5, "{south}");
    assert!((north.y - 50.0).abs() < 1e-5 && (south.y - 50.0).abs() < 1e-5);
    assert!(
        (north - south).length() >= 0.99,
        "a metre apart at the same place"
    );
    assert!(h_north.abs() < 1e-5 && (h_south - std::f32::consts::PI).abs() < 1e-5);
}

#[test]
fn no_lateral_offset_walks_on_the_centre_line_as_before() {
    let n = north_lane(2.0);
    let (p, _) = walker_on(0, 1, 0.0).pose(&n).unwrap();
    assert!(p.x.abs() < 1e-6);
}

#[test]
fn the_offset_is_capped_on_a_wide_lane_and_ignores_a_broken_width() {
    let wide = north_lane(20.0);
    let (p, _) = walker_on(0, 1, 1.0).pose(&wide).unwrap();
    assert!((p.x - MAX_OFFSET_M).abs() < 1e-5, "{p}");
    for width in [f32::NAN, f32::INFINITY, -3.0, 0.0] {
        let odd = north_lane(width);
        let (p, _) = walker_on(0, 1, 0.8).pose(&odd).unwrap();
        assert!(p.is_finite() && p.x.abs() < 1e-6, "width {width}: {p}");
    }
}

#[test]
fn new_walkers_draw_their_side_within_the_range_and_differ() {
    let n = north_lane(2.0);
    let mut rng = 12345;
    let sides: Vec<f32> = (0..200)
        .map(|_| Walker::new(&n, 0, 10.0, 1.4, &mut rng).unwrap().lateral)
        .collect();
    assert!(sides.iter().all(|l| (0.15..=0.85).contains(l)));
    let (lo, hi) = sides
        .iter()
        .fold((1.0f32, 0.0f32), |a, l| (a.0.min(*l), a.1.max(*l)));
    assert!(hi - lo > 0.5, "the sides vary: {lo}..{hi}");
    let mut again = 12345;
    let first = Walker::new(&n, 0, 10.0, 1.4, &mut again).unwrap().lateral;
    assert_eq!(first, sides[0], "the same seed gives the same side");
}
