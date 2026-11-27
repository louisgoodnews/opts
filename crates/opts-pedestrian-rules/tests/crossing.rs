//! Crossing rule: a walker holds at the stop line of a crossing whose light is at stop.
//!
//! Layout (hand-computed): sidewalk `A` runs north from (0,0) to (0,10); crossing `C` runs east
//! from (0,10) to (10,10) and is governed by light `KEY`. Lanes of one tile get the ids 0 (`A`)
//! and 1 (`C`) in insertion order.

use glam::Vec3;
use opts_pedestrian_rules::crossing::CrossingGates;
use opts_signals::{Aspect, SignalKey};
use opts_sim_pedestrian::pedestrian::lane::{FootLane, Travel};
use opts_sim_pedestrian::pedestrian::net::{PedestrianNet, Walker};

const KEY: SignalKey = (7, 0);

fn fixture() -> (PedestrianNet, CrossingGates) {
    let sidewalk = FootLane::new(
        vec![Vec3::ZERO, Vec3::new(0.0, 10.0, 0.0)],
        Travel::Both,
        2.0,
    )
    .unwrap();
    let crossing = FootLane::new(
        vec![Vec3::new(0.0, 10.0, 0.0), Vec3::new(10.0, 10.0, 0.0)],
        Travel::Both,
        3.0,
    )
    .unwrap();
    let mut net = PedestrianNet::default();
    net.insert_tile((0, 0), vec![sidewalk, crossing.clone()]);
    let mut gates = CrossingGates::default();
    gates.add_crossing(&crossing, KEY);
    (net, gates)
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
fn a_walker_one_step_short_of_a_red_crossing_holds() {
    let (net, mut gates) = fixture();
    gates.set_aspect(KEY, Aspect::Stop);
    // 0.2 m short of the stop line, a step of 1.4 m: it would reach the line.
    assert!(gates.holds(&net, &walker_at(0, 9.8, 1), 1.0));
}

#[test]
fn a_walker_holds_on_a_red_light_with_yellow_as_well() {
    let (net, mut gates) = fixture();
    // Red and yellow together is still a stop.
    gates.set_aspect(KEY, Aspect::Stop);
    assert!(gates.holds(&net, &walker_at(0, 9.8, 1), 0.5));
}

#[test]
fn a_walker_passes_on_green_yellow_and_dark() {
    let (net, mut gates) = fixture();
    for aspect in [Aspect::Go, Aspect::Yellow, Aspect::Off] {
        gates.set_aspect(KEY, aspect);
        assert!(!gates.holds(&net, &walker_at(0, 9.8, 1), 1.0), "{aspect:?}");
    }
}

#[test]
fn a_walker_far_from_the_line_keeps_walking_even_on_red() {
    let (net, mut gates) = fixture();
    gates.set_aspect(KEY, Aspect::Stop);
    // 5 m short of the line, a step of 1.4 m does not reach it.
    assert!(!gates.holds(&net, &walker_at(0, 5.0, 1), 1.0));
}

#[test]
fn a_walker_leaving_the_crossing_is_not_held_by_its_own_light() {
    let (net, mut gates) = fixture();
    gates.set_aspect(KEY, Aspect::Stop);
    // On the crossing at its far end, walking east: nothing beyond it is a crossing.
    assert!(!gates.holds(&net, &walker_at(1, 9.9, 1), 1.0));
    // Walking back west to the sidewalk: the sidewalk is not a crossing, although it meets the
    // crossing at the same junction point.
    assert!(!gates.holds(&net, &walker_at(1, 0.1, -1), 1.0));
}

#[test]
fn light_changes_take_effect_on_the_next_frame() {
    let (net, mut gates) = fixture();
    let w = walker_at(0, 9.8, 1);
    gates.set_aspect(KEY, Aspect::Stop);
    assert!(gates.holds(&net, &w, 1.0));
    gates.set_aspect(KEY, Aspect::Go);
    assert!(!gates.holds(&net, &w, 1.0));
}
