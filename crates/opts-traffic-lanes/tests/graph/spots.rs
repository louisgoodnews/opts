//! Tests of random and weighted spot selection and of shortest routes on the lane graph.

use opts_traffic_lanes::rng::Rng;

/// Lanes are drawn in proportion to their weight; a lane weighing 0 never.
#[test]
fn weighted_spots_follow_the_weights() {
    let g = LaneGraph::build(
        &[
            road(0.0, 0.0, 0.0, 200.0),
            road(10.0, 0.0, 0.0, 200.0),
            road(20.0, 0.0, 0.0, 200.0),
        ],
        &ConnectParams::default(),
    );
    let centre = DVec2::new(10.0, 100.0);
    let x_of = |id: LaneId| g.lane(id).unwrap().geometry.start.x;
    let mut rng = Rng::new(3);
    let mut counts = [0usize; 3];
    for _ in 0..3000 {
        let (id, _) = g
            .weighted_spot(centre, 0.0, 150.0, &mut rng, |l| match l.geometry.start.x {
                x if x < 5.0 => 3.0,
                x if x < 15.0 => 1.0,
                _ => 0.0,
            })
            .unwrap();
        counts[(x_of(id) / 10.0).round() as usize] += 1;
    }
    assert_eq!(counts[2], 0);
    let share = counts[0] as f64 / (counts[0] + counts[1]) as f64;
    assert!((share - 0.75).abs() < 0.04, "share {share}");
    assert!(g
        .weighted_spot(centre, 0.0, 150.0, &mut rng, |_| 0.0)
        .is_none());
}

#[test]
fn random_spots_lie_in_the_requested_band() {
    let g = LaneGraph::build(&[road(0.0, 0.0, 0.0, 400.0)], &ConnectParams::default());
    let mut rng = Rng::new(9);
    let mut found = 0;
    for _ in 0..50 {
        if let Some((id, s)) =
            g.random_spot(DVec2::new(0.0, 200.0), 50.0, 120.0, &mut rng, |_| true)
        {
            let d = g
                .lane(id)
                .unwrap()
                .geometry
                .point_at(s)
                .distance(DVec2::new(0.0, 200.0));
            assert!((50.0..=120.0).contains(&d));
            found += 1;
        }
    }
    assert!(found > 20, "only {found} spots");
    assert!(g
        .random_spot(DVec2::new(0.0, 200.0), 50.0, 120.0, &mut rng, |_| false)
        .is_none());
}

#[test]
fn shortest_route_picks_the_shorter_branch() {
    // A fork after lane 0: lane 1 is long, lane 2 short, both rejoin in lane 3 (straight
    // stubs are enough to test the search).
    let mut long = road(0.0, 50.0, 0.0, 200.0);
    long.source = 1;
    let short = road(0.0, 50.0, 0.0, 20.0);
    let end_long = road(0.0, 250.0, 0.0, 10.0);
    let end_short = road(0.0, 70.0, 0.0, 10.0);
    let g = LaneGraph::build(
        &[road(0.0, 0.0, 0.0, 50.0), long, short, end_long, end_short],
        &ConnectParams::default(),
    );
    assert_eq!(g.successors(LaneId(0)).len(), 2);
    let route = g.shortest_route(LaneId(0), LaneId(4)).unwrap();
    assert_eq!(route, [LaneId(0), LaneId(2), LaneId(4)]);
    assert!(g.shortest_route(LaneId(4), LaneId(0)).is_none());
}
