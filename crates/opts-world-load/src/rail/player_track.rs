//! Track placement of the player's tram and its route choice at the points.

use bevy::prelude::*;
use opts_sim::rail::{Branch, Train};

use super::player::RailPlayer;
use super::{RailNetwork, SWITCH_REACH_M};

/// Puts a rail vehicle that has no train yet on the nearest track; gives up (it stays a road
/// vehicle) when none is near. Returns false when the vehicle is not on a track this frame.
pub(super) fn place_on_track(
    rail: &mut RailPlayer,
    ch: &mut opts_sim::vehicle::chassis::ChassisState,
    net: &RailNetwork,
) -> bool {
    if net.0.lane_count() == 0 {
        return false; // tiles still streaming in
    }
    let p = Vec3::new(ch.position.x, ch.position.y, ch.elevation_z);
    match net.0.nearest(p, rail.snap_m) {
        Some((lane, s, dist)) => {
            let l = net.0.lane(lane).expect("lane exists");
            let (_, h) = l.at(s);
            let along = (h - ch.heading_rad).cos();
            let dir = if along >= 0.0 { 1 } else { -1 };
            tracing::info!(lane, s, dist, dir, "rail vehicle placed on the track");
            let mut train = Train::new(lane, s, dir);
            // The chassis position is the body's origin; the train point (front bogie)
            // lies one half span ahead of it.
            train.advance(rail.span, &net.0);
            rail.train = Some(train);
        }
        None => {
            let nearest = net.0.nearest(p, 5_000.0).map(|(_, _, d)| d);
            tracing::error!(
                at = %opts_bevy::position_tag(),
                nearest_track_m = ?nearest,
                "rail vehicle spawned more than {} m from any track; it stays a road vehicle",
                rail.snap_m
            );
            rail.gave_up = true;
            return false;
        }
    }
    true
}

/// Route choice at the next point (the `rail_branch_*` actions: the arrows) and the toggle of the
/// points ahead.
pub(super) fn choose_branch(
    train: &mut Train,
    (keyboard, action_keys): (
        &ButtonInput<KeyCode>,
        &Option<Res<opts_bevy::action_keys::ActionKeys>>,
    ),
    typing: bool,
    net: &RailNetwork,
) {
    let keys = opts_bevy::action_keys::resolve(action_keys);
    if typing {
    } else if keys.just_pressed(keyboard, "rail_branch_left") {
        train.branch = Branch::Left;
    } else if keys.just_pressed(keyboard, "rail_branch_straight") {
        train.branch = Branch::Straight;
    } else if keys.just_pressed(keyboard, "rail_branch_right") {
        train.branch = Branch::Right;
    } else if keys.just_pressed(keyboard, "rail_switch_toggle") {
        // The points ahead: the next way they offer, in the order straight, left, right.
        if let Some(ahead) = train.next_switch(&net.0, SWITCH_REACH_M) {
            let at = ahead
                .branches
                .iter()
                .position(|b| *b == train.branch)
                .unwrap_or(0);
            train.branch = ahead.branches[(at + 1) % ahead.branches.len()];
        }
    }
}
