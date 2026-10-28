//! Alighting walkers and script passenger-feed publication for the player's vehicle.

use std::collections::HashMap;

use bevy::math::Affine3A;
use bevy::prelude::*;
use opts_bevy::{Human, PassengerFeed};
use opts_sim::passengers::Event;

use crate::in_game::omsi_world::{advance_phase, apply_gait};

use super::rig::{self, PassengerRig, Walker};
use super::{facing, LEAVE_DISTANCE_M, OUTSIDE_SPEED_M_S};

/// Moves alighted passengers away from the vehicle and despawns them once they reach their target.
pub(super) fn step_leaving(
    rig: &mut PassengerRig,
    commands: &mut Commands,
    humans: &Query<&Human>,
    transforms: &mut Query<&mut Transform>,
    dt: f32,
) {
    let mut walking = Vec::new();
    for mut w in std::mem::take(&mut rig.leaving) {
        let d = w.position.distance(w.target);
        if d < 0.3 || w.walked_m > LEAVE_DISTANCE_M * 2.0 {
            commands.entity(w.entity).despawn();
            continue;
        }
        let dir = (w.target - w.position) / d;
        let step = OUTSIDE_SPEED_M_S * dt;
        w.position += dir * step.min(d);
        w.walked_m += step;
        if let Ok(mut t) = transforms.get_mut(w.entity) {
            t.translation = w.position;
            t.rotation = facing(dir);
        }
        if let Ok(h) = humans.get(w.entity) {
            w.phase = advance_phase(h, w.phase, OUTSIDE_SPEED_M_S, dt);
            apply_gait(h, w.leg_m, w.phase, transforms);
        }
        walking.push(w);
    }
    rig.leaving = walking;
}

/// Updates script callbacks and inserts [`PassengerFeed`] on each unit's script entity.
pub(super) fn publish_feeds(
    vehicle: Entity,
    rig: &PassengerRig,
    handle: Option<&opts_bevy::host_callbacks::PassengerCountsHandle>,
    entry_slots: &[(Entity, usize)],
    exit_slots: &[(Entity, usize)],
    commands: &mut Commands,
) {
    // What the scripts see: the counts of the callbacks and the inputs.
    if let Some(h) = handle {
        if let Ok(mut c) = h.0.write() {
            c.on_place = (0..rig.sim.cabin.places.len())
                .map(|i| rig.sim.on_place(i) as u32)
                .collect();
            c.on_link = (0..rig.sim.graph().links.len())
                .map(|i| rig.sim.on_link(i) as u32)
                .collect();
        }
    }
    // Each engine is told about its own doors; the head's also about the people inside.
    let mut feeds: HashMap<Entity, PassengerFeed> = HashMap::new();
    feeds.entry(vehicle).or_default().humans = rig.sim.humans_inside() as u32;
    for (i, r) in rig.sim.entry_requests().into_iter().enumerate() {
        let Some(&(store, n)) = entry_slots.get(i) else {
            continue;
        };
        let waiting = r
            || rig
                .approaching
                .iter()
                .any(|w| w.entry == i && w.position.distance(w.target) <= 0.4);
        if let Some(slot) = feeds.entry(store).or_default().entry_req.get_mut(n) {
            *slot = waiting;
        }
    }
    for (i, r) in rig.sim.exit_requests().into_iter().enumerate() {
        let Some(&(store, n)) = exit_slots.get(i) else {
            continue;
        };
        if let Some(slot) = feeds.entry(store).or_default().exit_req.get_mut(n) {
            *slot = r;
        }
    }
    // `try_insert`: a vehicle despawned in this frame takes its feed with it.
    for (store, feed) in feeds {
        commands.entity(store).try_insert(feed);
    }
}

/// Applies settled and alighted events from the passenger simulation.
pub(super) fn apply_sim_events(
    rig: &mut PassengerRig,
    frames: &[Affine3A],
    to_world: Affine3A,
    commands: &mut Commands,
    transforms: &mut Query<&mut Transform>,
) {
    for e in rig.sim.take_events() {
        match e {
            Event::Settled { id, place, seated } => {
                if let Some(s) = rig.actors.get_mut(&id) {
                    let pl = rig.sim.cabin.places[place];
                    s.seated = seated.then(|| rig::SeatPose {
                        rotation_rad: pl.rotation_deg.to_radians(),
                        height_m: pl.seat_height,
                    });
                }
            }
            Event::Alighted { id, exit } => {
                if let Some(s) = rig.actors.remove(&id) {
                    let out = rig
                        .sim
                        .cabin
                        .exits
                        .get(exit)
                        .and_then(|n| rig.sim.graph().points.get(*n))
                        .map_or(1.0, |p| if p.x >= 0.0 { 1.0 } else { -1.0 });
                    let frame = frames.get(s.unit).copied().unwrap_or(to_world);
                    let world = frame.transform_point3(s.local);
                    let away = frame
                        .transform_vector3(Vec3::new(out, 0.0, 0.0))
                        .normalize_or_zero();
                    commands.entity(s.entity).remove::<ChildOf>();
                    if let Ok(mut t) = transforms.get_mut(s.entity) {
                        t.translation = world;
                    }
                    rig.leaving.push(Walker {
                        entity: s.entity,
                        leg_m: s.leg_m,
                        phase: s.phase,
                        position: world,
                        target: world + away * LEAVE_DISTANCE_M,
                        entry: 0,
                        age: 0,
                        stops: 0,
                        walked_m: 0.0,
                    });
                }
            }
            _ => {}
        }
    }
}
