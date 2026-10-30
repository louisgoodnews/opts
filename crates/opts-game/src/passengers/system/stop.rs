//! Arriving at and leaving a bus stop, and groups sent by hand.

use bevy::math::Affine3A;
use bevy::prelude::*;
use opts_bevy::bus_stops::BusStopMarker;

use super::super::crowd::{
    stop_key, DEFAULT_RATE, FIRST_VISIT_WAIT_MIN, MAX_ACCUMULATE_MIN, MAX_CROWD,
    MAX_VISIBLE_WAITING, RATE_SETTING,
};
use super::super::rig::{PassengerRig, Unseen, Walker};
use super::PassengerEnv;

/// A group sent by hand: they appear beside the first entry, as at a stop.
pub(super) fn spawn_forced_group(
    e: &mut PassengerEnv,
    rig: &mut PassengerRig,
    frames: &[Affine3A],
    to_world: Affine3A,
) {
    let n = std::mem::take(&mut rig.force_group);
    rig.sim.arrive_at_stop();
    let (door_unit, door) = rig.door_outside(rig.force_entry);
    let frame = frames.get(door_unit).copied().unwrap_or(to_world);
    let side = if door.x >= 0.0 { 1.0 } else { -1.0 };
    for slot in 0..n {
        // Beyond the drawn crowd a group boards unseen, like the crowd of a real stop.
        let actor = (slot < MAX_VISIBLE_WAITING)
            .then(|| {
                e.pool
                    .spawn_actor(&mut e.commands, &mut e.bindposes, rig.rng.next_f32())
            })
            .flatten();
        let Some(actor) = actor else {
            let entry = rig.force_entry;
            rig.unseen.push(Unseen {
                entry,
                age: 18 + (rig.rng.next_f32() * 50.0) as u32,
                stops: 1 + (rig.rng.next_f32() * 3.0) as u32,
            });
            continue;
        };
        let local = door
            + Vec3::new(
                side * (1.6 + (slot % 3) as f32 * 0.8),
                0.0,
                ((slot / 3) as f32 - (n / 3) as f32 / 2.0) + (slot % 3) as f32 * 0.35,
            );
        let spot = frame.transform_point3(local);
        let entry = rig.nearest_entry(spot, frames);
        let age = actor.age.unwrap_or(18 + (rig.rng.next_f32() * 50.0) as u32);
        let stops_to_ride = 1 + (rig.rng.next_f32() * 3.0) as u32;
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
        });
    }
}

/// The vehicle stops at a bus stop: the crowd waiting there appears (those left behind by the last
/// visit and everybody who came since: the setting's rate times the map's passenger factor).
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
    rig.stops_served += 1;
    rig.sim.arrive_at_stop();
    if let Ok((_, g)) = e.stops.get(marker) {
        let tf = Transform::from(*g);
        // The crowd: those left behind by the last visit, and everybody who came since
        // (the setting's rate times the map's passenger factor for the hour).
        let key = stop_key(&tf);
        rig.stop_key = Some(key);
        let (last, left_behind) = e
            .crowds
            .0
            .get(&key)
            .copied()
            .unwrap_or((now - FIRST_VISIT_WAIT_MIN * 60.0, 0));
        let minutes = ((now - last) / 60.0).clamp(0.0, MAX_ACCUMULATE_MIN) as f32;
        let rate = settings
            .and_then(|s| s.raw(RATE_SETTING)?.trim().parse::<f32>().ok())
            .unwrap_or(DEFAULT_RATE);
        let factor = match (&job, &env) {
            (Some(job), Some(env)) => {
                crate::in_game::omsi_world::passenger_factor(job.passenger_curve(), env.hour())
            }
            _ => 1.0,
        };
        let arrivals = rig.rng.poisson(rate * factor * minutes / 60.0);
        let total = (left_behind + arrivals).min(MAX_CROWD);
        e.crowds.0.insert(key, (now, 0));
        rig.unseen.clear();
        for slot in 0..total {
            let spot = BusStopMarker::waiting_area(&tf, slot);
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
            });
        }
    }
}

/// The vehicle has left the stop: whoever did not get on stays for the next vehicle.
pub(super) fn leave_stop(e: &mut PassengerEnv, rig: &mut PassengerRig) {
    rig.stop = None;
    // Whoever did not get on stays at the stop for the next vehicle.
    if let Some(key) = rig.stop_key.take() {
        let left = (rig.approaching.len() + rig.unseen.len()) as u32;
        if let Some(c) = e.crowds.0.get_mut(&key) {
            c.1 = left;
        }
    }
    rig.unseen.clear();
    for w in rig.approaching.drain(..) {
        e.commands.entity(w.entity).despawn();
    }
}
