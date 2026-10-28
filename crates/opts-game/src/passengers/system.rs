//! Main update system for passengers of the player's vehicle.

use bevy::math::Affine3A;
use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy::prelude::*;
use opts_bevy::bus_stops::{nearest_stop, BusStopMarker};
use opts_bevy::{Human, HumanPose, PlayerVehicle, VehicleController, VehicleScripts};
use opts_sim::passengers::Doors;

use crate::in_game::omsi_world::{advance_phase, apply_gait, apply_rest_pose, PedestrianPool};

use super::crowd::{
    stop_key, StopCrowds, ACTOR_BUDGET, DEFAULT_RATE, FIRST_VISIT_WAIT_MIN, MAX_ACCUMULATE_MIN,
    MAX_CROWD, MAX_VISIBLE_WAITING, RATE_SETTING, UNSEEN_BOARD_S,
};
use super::rig::{PassengerRig, Shown, Unseen, Walker};
use super::{
    draw, facing, feed, AT_STOP_M, LEFT_M, MOVING_KMH, OUTSIDE_SPEED_M_S, SETTING, STOPPED_KMH,
};

/// Whether door `n` of kind `kind` (`entry`/`exit`) is open in the engine of `store`.
fn door_open(engines: &Query<&VehicleScripts>, (store, n): (Entity, usize), kind: &str) -> bool {
    engines
        .get(store)
        .ok()
        .and_then(|s| s.engine.lock().ok())
        .is_some_and(|e| e.get(&format!("pax_{kind}{n}_open")) > 0.5)
}

/// The doors the vehicle's scripts report: each unit's in the engine of its own scripts.
fn read_doors(
    engines: &Query<&VehicleScripts>,
    entries: &[(Entity, usize)],
    exits: &[(Entity, usize)],
) -> Doors {
    Doors {
        entries: entries
            .iter()
            .map(|&d| door_open(engines, d, "entry"))
            .collect(),
        exits: exits
            .iter()
            .map(|&d| door_open(engines, d, "exit"))
            .collect(),
    }
}

/// Runs the passengers of the player's vehicle.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn update_passengers(
    time: Res<Time>,
    mut commands: Commands,
    mut pool: ResMut<PedestrianPool>,
    mut bindposes: ResMut<Assets<SkinnedMeshInverseBindposes>>,
    mut rigs: Query<
        (
            Entity,
            &mut PassengerRig,
            &GlobalTransform,
            &VehicleController,
            Option<&opts_bevy::host_callbacks::PassengerCountsHandle>,
        ),
        With<PlayerVehicle>,
    >,
    engines: Query<&VehicleScripts>,
    stops: Query<(Entity, &GlobalTransform), With<BusStopMarker>>,
    humans: Query<&Human>,
    globals: Query<&GlobalTransform>,
    mut transforms: Query<&mut Transform>,
    settings: Option<Res<crate::settings::GameSettings>>,
    mut crowds: ResMut<StopCrowds>,
    job: Option<Res<crate::in_game::omsi_world::WorldLoadJob>>,
    env: Option<Res<opts_bevy::SessionEnvironment>>,
) {
    let dt = time.delta_secs().min(0.1);
    if dt <= 0.0 {
        return;
    }
    let on = settings
        .as_ref()
        .and_then(|s| s.raw(SETTING))
        .is_none_or(|v| v.trim() != "false");
    for (vehicle, mut rig, vgt, controller, handle) in &mut rigs {
        let rig = &mut *rig;
        if !on {
            // Switched off: everybody goes at once.
            for (_, s) in rig.actors.drain() {
                commands.entity(s.entity).despawn();
            }
            for w in rig.approaching.drain(..).chain(rig.leaving.drain(..)) {
                commands.entity(w.entity).despawn();
            }
            rig.sim.clear();
            rig.stop = None;
            continue;
        }
        pool.want();
        let to_world = vgt.affine();
        let frames: Vec<Affine3A> = rig
            .units
            .iter()
            .map(|u| {
                globals
                    .get(u.root)
                    .map_or(to_world, GlobalTransform::affine)
            })
            .collect();
        let speed = controller.forward_speed_km_h().abs();
        let here = vgt.translation();
        let markers: Vec<(Entity, Transform)> = stops
            .iter()
            .map(|(e, g)| (e, Transform::from_translation(g.translation())))
            .collect();
        let nearest = nearest_stop(markers.iter().map(|(e, t)| (*e, t)), here, LEFT_M);
        // A group sent by hand: they appear beside the first entry, as at a stop.
        if rig.force_group > 0 {
            let n = std::mem::take(&mut rig.force_group);
            rig.sim.arrive_at_stop();
            let (door_unit, door) = rig.door_outside(rig.force_entry);
            let frame = frames.get(door_unit).copied().unwrap_or(to_world);
            let side = if door.x >= 0.0 { 1.0 } else { -1.0 };
            for slot in 0..n {
                // Beyond the drawn crowd a group boards unseen, like the crowd of a real stop.
                let actor = (slot < MAX_VISIBLE_WAITING)
                    .then(|| pool.spawn_actor(&mut commands, &mut bindposes, rig.rng.next_f32()))
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
                let entry = rig.nearest_entry(spot, &frames);
                let age = actor.age.unwrap_or(18 + (rig.rng.next_f32() * 50.0) as u32);
                let stops_to_ride = 1 + (rig.rng.next_f32() * 3.0) as u32;
                commands
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
        // Arriving at and leaving a stop.
        match (rig.stop, nearest) {
            (None, Some((marker, d))) if d <= AT_STOP_M && speed <= STOPPED_KMH => {
                rig.stop = Some(marker);
                rig.stops_served += 1;
                rig.sim.arrive_at_stop();
                if let Ok((_, g)) = stops.get(marker) {
                    let tf = Transform::from(*g);
                    // The crowd: those left behind by the last visit, and everybody who came since
                    // (the setting's rate times the map's passenger factor for the hour).
                    let key = stop_key(&tf);
                    rig.stop_key = Some(key);
                    let now = time.elapsed_secs_f64();
                    let (last, left_behind) = crowds
                        .0
                        .get(&key)
                        .copied()
                        .unwrap_or((now - FIRST_VISIT_WAIT_MIN * 60.0, 0));
                    let minutes = ((now - last) / 60.0).clamp(0.0, MAX_ACCUMULATE_MIN) as f32;
                    let rate = settings
                        .as_ref()
                        .and_then(|s| s.raw(RATE_SETTING)?.trim().parse::<f32>().ok())
                        .unwrap_or(DEFAULT_RATE);
                    let factor = match (&job, &env) {
                        (Some(job), Some(env)) => crate::in_game::omsi_world::passenger_factor(
                            job.passenger_curve(),
                            env.hour(),
                        ),
                        _ => 1.0,
                    };
                    let arrivals = rig.rng.poisson(rate * factor * minutes / 60.0);
                    let total = (left_behind + arrivals).min(MAX_CROWD);
                    crowds.0.insert(key, (now, 0));
                    rig.unseen.clear();
                    for slot in 0..total {
                        let spot = BusStopMarker::waiting_area(&tf, slot);
                        let entry = rig.nearest_entry(spot, &frames);
                        let age_roll = 18 + (rig.rng.next_f32() * 50.0) as u32;
                        let stops_to_ride = 1 + (rig.rng.next_f32() * 6.0) as u32;
                        // The first ones are drawn; the rest of the crowd boards without a figure.
                        let actor = (slot < MAX_VISIBLE_WAITING)
                            .then(|| {
                                pool.spawn_actor(&mut commands, &mut bindposes, rig.rng.next_f32())
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
                        commands
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
            (Some(_), near) => {
                let gone = near.is_none_or(|(_, d)| d > LEFT_M) || speed > MOVING_KMH;
                if gone {
                    rig.stop = None;
                    // Whoever did not get on stays at the stop for the next vehicle.
                    if let Some(key) = rig.stop_key.take() {
                        let left = (rig.approaching.len() + rig.unseen.len()) as u32;
                        if let Some(c) = crowds.0.get_mut(&key) {
                            c.1 = left;
                        }
                    }
                    rig.unseen.clear();
                    for w in rig.approaching.drain(..) {
                        commands.entity(w.entity).despawn();
                    }
                }
            }
            _ => {}
        }
        // People walking up to the doors; they board when the door is open.
        let (entry_slots, exit_slots) = rig.door_slots();
        let doors = read_doors(&engines, &entry_slots, &exit_slots);
        let mut still = Vec::new();
        for mut w in std::mem::take(&mut rig.approaching) {
            w.target = rig.door_world(w.entry, &frames);
            let d = w.position.distance(w.target);
            let step = OUTSIDE_SPEED_M_S * dt;
            if d > 0.25 {
                let dir = (w.target - w.position) / d;
                w.position += dir * step.min(d);
                if let Ok(mut t) = transforms.get_mut(w.entity) {
                    t.translation = w.position;
                    t.rotation = facing(dir);
                }
                if let Ok(h) = humans.get(w.entity) {
                    w.phase = advance_phase(h, w.phase, OUTSIDE_SPEED_M_S, dt);
                    apply_gait(h, w.leg_m, w.phase, &mut transforms);
                }
                still.push(w);
            } else if doors.entries.get(w.entry).copied().unwrap_or(false)
                && rig.sim.passengers().len() < rig.sim.capacity()
            {
                if let Some(id) = rig.sim.board(w.age, w.stops, w.entry) {
                    // Beyond the figure budget a person boards unseen.
                    if rig.actors.len() >= ACTOR_BUDGET {
                        commands.entity(w.entity).despawn();
                        continue;
                    }
                    let unit = rig
                        .sim
                        .passengers()
                        .iter()
                        .find(|p| p.id == id)
                        .map_or(0, |p| p.unit);
                    let frame = frames.get(unit).copied().unwrap_or(to_world);
                    let local = frame.inverse().transform_point3(w.position);
                    let parent = rig.units.get(unit).map_or(vehicle, |u| u.root);
                    commands.entity(w.entity).insert(ChildOf(parent));
                    if let Ok(mut t) = transforms.get_mut(w.entity) {
                        t.translation = local;
                    }
                    rig.actors.insert(
                        id,
                        Shown {
                            unit,
                            entity: w.entity,
                            leg_m: w.leg_m,
                            phase: w.phase,
                            local,
                            seated: None,
                        },
                    );
                } else {
                    commands.entity(w.entity).despawn();
                }
            } else {
                // Waiting for the door: stand still.
                if let Ok(h) = humans.get(w.entity) {
                    apply_rest_pose(h, HumanPose::Standing, &mut transforms);
                }
                still.push(w);
            }
        }
        rig.approaching = still;
        // The rest of the crowd boards through the open doors, one person per door in turn, while
        // there is room.
        rig.unseen_timer += dt;
        if rig.unseen_timer >= UNSEEN_BOARD_S && !rig.unseen.is_empty() {
            rig.unseen_timer = 0.0;
            for entry in 0..doors.entries.len() {
                if !doors.entries[entry] || rig.sim.passengers().len() >= rig.sim.capacity() {
                    continue;
                }
                if let Some(i) = rig.unseen.iter().position(|u| u.entry == entry) {
                    let u = rig.unseen.remove(i);
                    rig.sim.board(u.age, u.stops, u.entry);
                }
            }
        }
        // The passengers.
        rig.sim.step(dt, &doors);
        feed::apply_sim_events(rig, &frames, to_world, &mut commands, &mut transforms);
        // Draw the passengers inside.
        draw::draw_inside(rig, &mut commands, &humans, &mut transforms, dt);
        // Those who got off walk away and vanish.
        feed::step_leaving(rig, &mut commands, &humans, &mut transforms, dt);
        feed::publish_feeds(
            vehicle,
            rig,
            handle,
            &entry_slots,
            &exit_slots,
            &mut commands,
        );
    }
}
