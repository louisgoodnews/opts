//! Separation at its edges: the two-metre follow distance is inclusive, a follower takes the
//! lowest speed of every walker it follows, and a walker on a lane the net does not have keeps
//! its own speed.
//!
//! Layout as in `separation.rs`: one lane runs north from (0,0) to (0,100), 2 m wide.

use glam::Vec3;
use opts_pedestrian_rules::separation::separated_speeds;
use opts_sim_pedestrian::pedestrian::lane::{FootLane, Travel};
use opts_sim_pedestrian::pedestrian::net::{PedestrianNet, Walker};

fn net() -> PedestrianNet {
    let lane = FootLane::new(
        vec![Vec3::ZERO, Vec3::new(0.0, 100.0, 0.0)],
        Travel::Both,
        2.0,
    )
    .unwrap();
    let mut n = PedestrianNet::default();
    n.insert_tile((0, 0), vec![lane]);
    n
}

fn walker(s: f32, dir: i8, speed: f32, lateral: f32) -> Walker {
    Walker {
        lane: 0,
        s,
        dir,
        speed,
        phase: 0.0,
        lateral,
    }
}

#[test]
fn a_walker_exactly_two_metres_ahead_is_still_followed() {
    let net = net();
    let ws = [walker(50.0, 1, 1.4, 0.5), walker(52.0, 1, 0.9, 0.5)];
    // The follow distance is inclusive: the follower drops to 0.9 m/s.
    assert_eq!(separated_speeds(&ws, &net), vec![0.9, 0.9]);
}

#[test]
fn a_follower_takes_the_lowest_speed_of_the_walkers_ahead() {
    let net = net();
    let ws = [
        walker(50.0, 1, 1.4, 0.5),
        walker(51.0, 1, 1.0, 0.5),
        walker(51.5, 1, 0.6, 0.5),
    ];
    // Walker at 50 follows both ahead of it; walker at 51 follows the one at 51.5.
    assert_eq!(separated_speeds(&ws, &net), vec![0.6, 0.6, 0.6]);
}

#[test]
fn a_faster_walker_ahead_does_not_speed_the_follower_up() {
    let net = net();
    let ws = [walker(50.0, 1, 0.5, 0.5), walker(51.0, 1, 1.4, 0.5)];
    assert_eq!(separated_speeds(&ws, &net), vec![0.5, 1.4]);
}

#[test]
fn a_walker_on_a_lane_the_net_does_not_have_keeps_its_speed() {
    let net = net();
    let lost = Walker {
        lane: 9,
        ..walker(50.5, 1, 0.3, 0.5)
    };
    let ws = [walker(50.0, 1, 1.4, 0.5), lost];
    // The walker without a pose is neither followed nor follows; it keeps its own speed.
    assert_eq!(separated_speeds(&ws, &net), vec![1.4, 0.3]);
}
