//! Crossing registry as the tile wiring uses it: lanes are registered and removed per tile, and
//! a light's stop lines are kept apart from the stop lines of other lights.
//!
//! Layout (hand-computed): sidewalk `A` runs north from (0,0) to (0,10); crossing `C` runs east
//! from (0,10) to (10,10); sidewalk `B` runs north from (10,10) to (10,20). Lanes of one tile get
//! the ids 0 (`A`), 1 (`C`) and 2 (`B`) in insertion order. A walker of `B` going south reaches
//! the crossing from its far end.

use glam::Vec3;
use opts_pedestrian_rules::crossing::CrossingGates;
use opts_signals::{Aspect, SignalKey};
use opts_sim_pedestrian::pedestrian::lane::{FootLane, Travel};
use opts_sim_pedestrian::pedestrian::net::{PedestrianNet, Walker};

const KEY: SignalKey = (7, 0);
const OTHER: SignalKey = (8, 0);

fn lane(points: [Vec3; 2], width: f32) -> FootLane {
    FootLane::new(points.to_vec(), Travel::Both, width).unwrap()
}

fn sidewalk_a() -> FootLane {
    lane([Vec3::ZERO, Vec3::new(0.0, 10.0, 0.0)], 2.0)
}

fn crossing() -> FootLane {
    lane([Vec3::new(0.0, 10.0, 0.0), Vec3::new(10.0, 10.0, 0.0)], 3.0)
}

fn sidewalk_b() -> FootLane {
    lane([Vec3::new(10.0, 10.0, 0.0), Vec3::new(10.0, 20.0, 0.0)], 2.0)
}

fn net() -> PedestrianNet {
    let mut net = PedestrianNet::default();
    net.insert_tile((0, 0), vec![sidewalk_a(), crossing(), sidewalk_b()]);
    net
}

fn walker_at(lane: u32, s: f32, dir: i8) -> Walker {
    Walker {
        lane,
        s,
        dir,
        speed: 1.4,
        phase: 0.0,
        lateral: 0.5,
    }
}

#[test]
fn a_walker_coming_from_the_far_side_is_held_too() {
    let net = net();
    let mut gates = CrossingGates::default();
    gates.add_crossing(&crossing(), KEY);
    gates.set_aspect(KEY, Aspect::Stop);
    // Sidewalk B walks south onto the crossing's east end, 0.2 m short of the junction.
    assert!(gates.holds(&net, &walker_at(2, 0.2, -1), 1.0));
}

#[test]
fn a_crossing_removed_from_the_registry_no_longer_holds_walkers() {
    let net = net();
    let mut gates = CrossingGates::default();
    gates.add_crossing(&crossing(), KEY);
    gates.set_aspect(KEY, Aspect::Stop);
    assert!(gates.holds(&net, &walker_at(0, 9.8, 1), 1.0));
    gates.remove_crossing(&crossing(), KEY);
    assert!(!gates.holds(&net, &walker_at(0, 9.8, 1), 1.0));
}

#[test]
fn removing_one_light_keeps_the_stop_line_of_another_light_at_the_same_points() {
    let net = net();
    let mut gates = CrossingGates::default();
    gates.add_crossing(&crossing(), KEY);
    gates.add_crossing(&crossing(), OTHER);
    gates.set_aspect(OTHER, Aspect::Stop);
    // The stop lines now belong to OTHER, so removing KEY leaves them in place.
    gates.remove_crossing(&crossing(), KEY);
    assert!(gates.holds(&net, &walker_at(0, 9.8, 1), 1.0));
    gates.remove_crossing(&crossing(), OTHER);
    assert!(!gates.holds(&net, &walker_at(0, 9.8, 1), 1.0));
}

#[test]
fn a_light_that_was_never_set_leaves_its_crossing_open() {
    let net = net();
    let mut gates = CrossingGates::default();
    gates.add_crossing(&crossing(), KEY);
    // No set_aspect call: the registry default is a light that is not at stop.
    assert!(!gates.holds(&net, &walker_at(0, 9.8, 1), 1.0));
}

#[test]
fn a_walker_on_a_lane_the_net_does_not_have_is_not_held() {
    let net = net();
    let mut gates = CrossingGates::default();
    gates.add_crossing(&crossing(), KEY);
    gates.set_aspect(KEY, Aspect::Stop);
    assert!(!gates.holds(&net, &walker_at(9, 9.8, 1), 1.0));
}
