//! Arriving at and leaving a bus stop, and groups sent by hand.

use bevy::math::Affine3A;
use bevy::prelude::*;
use opts_bevy::bus_stops::BusStopMarker;
use opts_sim::passengers::Destination;

use super::super::crowd::{
    stop_key, Crowd, DEFAULT_RATE, FIRST_VISIT_WAIT_MIN, MAX_ACCUMULATE_MIN, MAX_CROWD,
    MAX_VISIBLE_WAITING, RATE_SETTING,
};
use super::super::destinations::draw_dest;
use super::super::rig::{PassengerRig, Unseen, Walker};
use super::PassengerEnv;

/// The people who gather at a stop: those left behind by the last visit, and the arrivals since
/// (the setting's rate times the map's passenger factor for the hour). Each has a destination drawn
/// from the stop's trips.
fn gather(
    rig: &mut PassengerRig,
    (left, arrivals): (Vec<Option<Destination>>, u32),
) -> Vec<Option<Destination>> {
    let mut crowd = left;
    for _ in 0..arrivals {
        crowd.push(draw_dest(rig));
    }
    crowd.truncate(MAX_CROWD as usize);
    crowd
}

/// The rate of arrivals at a stop (per hour): the setting or the default, times the passenger
/// factor of the hour.
fn arrival_rate(
    settings: Option<&crate::settings::GameSettings>,
    job: Option<&crate::in_game::omsi_world::WorldLoadJob>,
    env: Option<&opts_bevy::SessionEnvironment>,
) -> f32 {
    let rate = settings
        .and_then(|s| s.raw(RATE_SETTING)?.trim().parse::<f32>().ok())
        .unwrap_or(DEFAULT_RATE);
    let factor = match (job, env) {
        (Some(job), Some(env)) => {
            crate::in_game::omsi_world::passenger_factor(job.passenger_curve(), env.hour())
        }
        _ => 1.0,
    };
    rate * factor
}

/// The vehicle stops at a bus stop: the crowd waiting there appears, each with a destination.
pub(super) fn arrive_at_stop(
    e: &mut PassengerEnv,
    rig: &mut PassengerRig,
    (marker, now): (Entity, f64),
    frames: &[Affine3A],
    (settings, job, env): (
        Option<&crate::settings::GameSettings>,
        Option<&crate::in_game::omsi_world::WorldLoadJob>,
        Option<&opts_bevy::SessionEnvironment>,
    ),
) {
    rig.stop = Some(marker);
    rig.stop_id = e.markers.get(marker).ok().map(|m| m.id_code);
    rig.stops_served += 1;
    rig.sim.arrive_at_stop();
    let Ok((_, g)) = e.stops.get(marker) else {
        return;
    };
    let tf = Transform::from(*g);
    let key = stop_key(&tf);
    rig.stop_key = Some(key);
    let crowd = e.crowds.0.remove(&key).unwrap_or_else(|| Crowd {
        since: now - FIRST_VISIT_WAIT_MIN * 60.0,
        left: Vec::new(),
    });
    let minutes = ((now - crowd.since) / 60.0).clamp(0.0, MAX_ACCUMULATE_MIN) as f32;
    let arrivals = rig
        .rng
        .poisson(arrival_rate(settings, job, env) * minutes / 60.0);
    let dests = gather(rig, (crowd.left, arrivals));
    e.crowds.0.insert(
        key,
        Crowd {
            since: now,
            left: Vec::new(),
        },
    );
    rig.unseen.clear();
    spawn_crowd(e, rig, &tf, frames, dests);
}

/// Draws the first people of the crowd at their places and counts the rest as unseen.
fn spawn_crowd(
    e: &mut PassengerEnv,
    rig: &mut PassengerRig,
    tf: &Transform,
    frames: &[Affine3A],
    dests: Vec<Option<Destination>>,
) {
    for (slot, dest) in dests.into_iter().enumerate() {
        let slot = slot as u32;
        let spot = BusStopMarker::waiting_area(tf, slot);
        let entry = rig.nearest_entry(spot, frames);
        let age_roll = 18 + (rig.rng.next_f32() * 50.0) as u32;
        let stops_to_ride = 1 + (rig.rng.next_f32() * 6.0) as u32;
        // The first ones are drawn; the rest of the crowd boards without a figure.
        let actor = (slot < MAX_VISIBLE_WAITING)
            .then(|| {
                e.pool
                    .spawn_actor(&mut e.commands, &mut e.bindposes, rig.rng.next_f32())
            })
            .flatten();
        let Some(actor) = actor else {
            rig.unseen.push(Unseen {
                entry,
                age: age_roll,
                stops: stops_to_ride,
                dest,
            });
            continue;
        };
        let age = actor.age.unwrap_or(age_roll);
        e.commands
            .entity(actor.entity)
            .insert(Transform::from_translation(spot));
        rig.approaching.push(Walker {
            entity: actor.entity,
            leg_m: actor.leg_m,
            phase: 0.0,
            position: spot,
            target: Vec3::ZERO,
            entry,
            age,
            stops: stops_to_ride,
            walked_m: 0.0,
            dest,
        });
    }
}

/// The vehicle has left the stop: whoever did not get on stays for the next vehicle, with their
/// destinations.
pub(super) fn leave_stop(e: &mut PassengerEnv, rig: &mut PassengerRig) {
    rig.stop = None;
    rig.stop_id = None;
    if let Some(key) = rig.stop_key.take() {
        let left: Vec<Option<Destination>> = rig
            .approaching
            .iter()
            .map(|w| w.dest.clone())
            .chain(rig.unseen.iter().map(|u| u.dest.clone()))
            .collect();
        if let Some(c) = e.crowds.0.get_mut(&key) {
            c.left = left;
        }
    }
    rig.unseen.clear();
    for w in rig.approaching.drain(..) {
        e.commands.entity(w.entity).despawn();
    }
}
