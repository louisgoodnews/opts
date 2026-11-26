//! Points: routes, the next points ahead, gated lanes and the train that is never stranded.

use super::*;

#[test]
fn a_route_decides_at_the_point_and_the_shortest_path_finds_it() {
    let n = net();
    assert_eq!(n.shortest_path(0, 1, 2, 1000.0), Some(vec![(2, 1)]));
    assert_eq!(n.shortest_path(0, 1, 1, 1000.0), Some(vec![(1, 1)]));
    assert_eq!(n.shortest_path(0, 1, 0, 1000.0), Some(Vec::new()));
    assert_eq!(n.shortest_path(0, 1, 2, 50.0), None, "beyond the limit");
    // Against the grain no lane leads to lane 1 from the start of lane 0 within reach of
    // the branch: walking backwards from lane 0 finds nothing.
    assert_eq!(n.shortest_path(0, -1, 2, 1000.0), None);
    let mut t = Train::new(0, 90.0, 1);
    t.route = [2].into();
    assert_eq!(t.advance(30.0, &n), Moved::Full);
    assert_eq!(t.lane, 2, "the route overrides the straight way");
    assert!(t.route.is_empty());
}

/// Lanes 1 (straight on) and 2 (to the north-east) belong to the points `K` in positions 0
/// and 1.
fn gated_net() -> PedestrianNet {
    use opts_sim_pedestrian::Gate;
    let gate = |position| Gate {
        switch: 7,
        position,
    };
    let mut n = PedestrianNet::default();
    n.insert_tile(
        (0, 0),
        vec![
            lane(Vec3::ZERO, Vec3::new(0.0, 100.0, 0.0)),
            lane(Vec3::new(0.0, 100.0, 0.0), Vec3::new(0.0, 200.0, 0.0)).with_gate(gate(0)),
            lane(Vec3::new(0.0, 100.0, 0.0), Vec3::new(50.0, 150.0, 0.0)).with_gate(gate(1)),
        ],
    );
    n
}

#[test]
fn a_train_takes_the_open_way_whatever_it_wants() {
    let mut n = gated_net();
    let mut t = Train::new(0, 90.0, 1);
    t.branch = Branch::Right;
    n.set_switch_position(7, 0);
    t.advance(30.0, &n);
    assert_eq!(
        t.lane, 1,
        "points set to straight: the diverging lane is closed"
    );
    let mut t = Train::new(0, 90.0, 1);
    t.branch = Branch::Straight;
    n.set_switch_position(7, 1);
    t.advance(30.0, &n);
    assert_eq!(
        t.lane, 2,
        "points set to diverging: the straight lane is closed"
    );
}

#[test]
fn the_points_ahead_name_the_lane_the_train_wants_even_when_it_is_closed() {
    let mut n = gated_net();
    n.set_switch_position(7, 0);
    let mut t = Train::new(0, 50.0, 1);
    t.branch = Branch::Right;
    let ahead = t.next_switch(&n, 100.0).expect("points ahead");
    assert_eq!(ahead.wanted, 2);
    assert!(n.is_closed(ahead.wanted));
    t.branch = Branch::Straight;
    assert_eq!(t.next_switch(&n, 100.0).unwrap().wanted, 1);
}

#[test]
fn a_train_is_never_stranded_when_every_way_is_closed() {
    let mut n = gated_net();
    n.set_switch_position(7, 5);
    let mut t = Train::new(0, 90.0, 1);
    assert_eq!(t.advance(30.0, &n), Moved::Full);
    assert_eq!(t.lane, 1);
}
