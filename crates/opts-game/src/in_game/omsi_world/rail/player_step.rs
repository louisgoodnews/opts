//! Per-frame step of the player's tram: pedals and direction, script forces, and the
//! longitudinal dynamics along the track.

use bevy::prelude::*;
use opts_sim::rail::{Moved, RailInput};
use opts_sim::GearPosition;

use super::player::RailPlayer;
use super::player_track::{choose_branch, place_on_track};
use super::{unit_frame, RailBody, RailNetwork, STANDSTILL_M_S};

/// The pull and brake the vehicle's own scripts command when they write `M_Wheel`: torque at the
/// wheel radius is the pull, their brake force the brake (the constants are the fallback).
fn script_forces(
    drive: Option<opts_sim::vehicle::ScriptDrive>,
    rail: &RailPlayer,
) -> Option<opts_sim::rail::ScriptForces> {
    drive.map(|d| opts_sim::rail::ScriptForces {
        // Forward the script pulls with positive torque. Backing up the direction is
        // ours, so its torque counts whatever its sign: a bus-style script reverses the
        // sign, the NF6D's reverse-drive switch keeps it.
        traction_n: if rail.backing {
            d.wheel_torque_nm.abs()
        } else {
            d.wheel_torque_nm
        } / rail.wheel_radius_m,
        brake_n: d.brake_force_n,
    })
}

/// The throttle (in drive), the service brake and whether the vehicle is held: a rail vehicle has
/// no parking brake of its own, so its controls ignore it.
fn pedals(controller: &opts_bevy::VehicleController, has_controls: bool) -> (f32, f32, bool) {
    let i = &controller.sim.inputs;
    (
        if i.gear() == GearPosition::Drive {
            i.throttle()
        } else {
            0.0
        },
        i.service_brake(),
        // A rail vehicle has no parking brake of its own: its controls ignore it.
        i.is_parking_brake_engaged() && !has_controls,
    )
}

/// Holds the player's tram on the track and runs its longitudinal dynamics (fixed step, after the
/// road physics, before the articulation of the following units).
pub fn rail_player_step(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    action_keys: Option<Res<opts_bevy::action_keys::ActionKeys>>,
    modal: Option<Res<opts_bevy::reporting::ModalActive>>,
    menu: Option<Res<opts_bevy::reporting::MenuModalActive>>,
    net: Res<RailNetwork>,
    mut players: Query<RailBody>,
) {
    // Typing in a modal must not switch points.
    let typing = modal.is_some_and(|m| m.0) || menu.is_some_and(|m| m.0);
    let dt = time.delta_secs();
    if dt <= 0.0 {
        return;
    }
    for (mut controller, mut rail, controls, cab, consist) in &mut players {
        if rail.gave_up {
            continue;
        }
        let (traction, brake, hold) = pedals(&controller, controls.is_some());
        // The direction of travel changes only at a standstill; a backing vehicle keeps facing
        // the way it faced and its head moves back along the track it ran over.
        // Driven from the cab at the far end (a tram's head B, the last train of a traction) the
        // vehicle's forward is the head's backward: the consist moves back along its track.
        let cab_turned = opts_bevy::cab_switch::cab_is_turned(cab, consist);
        if let Some(c) = controls {
            if rail.speed < STANDSTILL_M_S {
                rail.backing = (c.direction < 0) != cab_turned;
            }
        }
        // The vehicle's own scripts drive it when they write `M_Wheel`: torque at the wheel
        // radius is the pull, their brake force the brake (the constants are the fallback).
        let script = script_forces(controller.sim.script_drive, &rail);
        let ch = &mut controller.sim.chassis;
        if rail.train.is_none() && !place_on_track(&mut rail, ch, &net) {
            continue;
        }
        let input = RailInput {
            traction,
            brake,
            emergency: false,
            hold,
            direction: 1,
            script,
        };
        let Some(train) = rail.train.as_mut() else {
            continue;
        };
        choose_branch(train, (&keyboard, &action_keys), typing, &net);
        let pose_ahead = train.pose(&net.0);
        let grade = match (pose_ahead, train.pose_behind(&net.0, 6.0)) {
            (Some((a, _)), Some((b, _))) => (a.z - b.z) / 6.0,
            _ => 0.0,
        };
        let speed = rail.dynamics.step(rail.speed, grade, &input, dt);
        rail.speed = speed;
        let backing = rail.backing;
        let Some(train) = rail.train.as_mut() else {
            continue;
        };
        let moved = if backing {
            train.retreat(speed * dt, &net.0)
        } else {
            train.advance(speed * dt, &net.0)
        };
        if moved == Moved::EndOfTrack {
            rail.speed = 0.0;
        }
        let Some(train) = rail.train.as_ref() else {
            continue;
        };
        // The chassis is the head body: midpoint and heading of its two bogies on the track.
        if let Some((mid, heading, _)) = unit_frame(train, &net.0, rail.span, rail.span, 0.0) {
            ch.position = Vec2::new(mid.x, mid.y);
            ch.heading_rad = heading;
            ch.elevation_z = mid.z;
            ch.local_velocity = Vec2::new(0.0, rail.speed);
            ch.yaw_rate_rad_s = 0.0;
        }
    }
}
