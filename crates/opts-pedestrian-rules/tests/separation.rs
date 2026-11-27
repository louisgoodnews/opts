//! Separation: a walker keeps no faster than a walker ahead of it on its own path.
//!
//! Layout (hand-computed): one lane runs north from (0,0) to (0,100), 2 m wide, so its half
//! width is 1 m and a walker with lateral `L` stands `L` metres east of the centre line.
//! Walkers are placed by arc length `s`; "ahead" is a larger `s` for direction `+1`.

use opts_pedestrian_rules::separation::separated_speeds;
use opts_sim_pedestrian::pedestrian::lane::{FootLane, Travel};
use opts_sim_pedestrian::pedestrian::net::{PedestrianNet, Walker};
use glam::Vec3;

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

/// A walker on lane 0 at `s`, walking `dir`, at `speed`, `lateral` (see the module docs).
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
fn a_walker_follows_the_slower_walker_one_metre_ahead_in_its_band() {
    let net = net();
    let ws = [walker(50.0, 1, 1.4, 0.5), walker(51.0, 1, 1.0, 0.5)];
    // The follower drops to 1.0 m/s; the leader is ahead of nobody and keeps its speed.
    assert_eq!(separated_speeds(&ws, &net), vec![1.0, 1.0]);
}

#[test]
fn a_walker_beyond_two_metres_ahead_is_not_followed() {
    let net = net();
    let ws = [walker(50.0, 1, 1.4, 0.5), walker(53.0, 1, 0.9, 0.5)];
    assert_eq!(separated_speeds(&ws, &net), vec![1.4, 0.9]);
}

#[test]
fn a_walker_in_another_band_is_not_followed() {
    let net = net();
    // Offsets 0 m and 0.85 m: 0.85 m sideways is more than 0.6 m.
    let ws = [walker(50.0, 1, 1.4, 0.0), walker(51.0, 1, 0.2, 0.85)];
    assert_eq!(separated_speeds(&ws, &net), vec![1.4, 0.2]);
}

#[test]
fn a_walker_within_the_band_at_half_a_metre_sideways_is_followed() {
    let net = net();
    // Offsets 0 m and 0.5 m: 0.5 m sideways is within 0.6 m, and the walker is 1.5 m ahead.
    let ws = [walker(50.0, 1, 1.4, 0.0), walker(51.5, 1, 0.6, 0.5)];
    assert_eq!(separated_speeds(&ws, &net), vec![0.6, 0.6]);
}

#[test]
fn a_walker_going_the_other_way_is_not_followed() {
    let net = net();
    let ws = [walker(50.0, 1, 1.4, 0.0), walker(51.0, -1, 0.3, 0.0)];
    assert_eq!(separated_speeds(&ws, &net), vec![1.4, 0.3]);
}

#[test]
fn a_walker_behind_is_not_followed() {
    let net = net();
    let ws = [walker(50.0, 1, 1.4, 0.5), walker(49.0, 1, 0.3, 0.5)];
    assert_eq!(separated_speeds(&ws, &net), vec![1.4, 0.3]);
}
