//! The passengers of the player's vehicle in the world (M-0101).
//!
//! The logic is [`opts_sim::passengers`] (the cabin, the doors, the tickets); this module is its
//! place in the game: it finds the bus stop the bus stands at, lets people walk up to the open
//! doors and in, shows them inside (walking, seated or standing), lets them get off and walk
//! away, and tells the vehicle's scripts how many people there are and which doors are wanted
//! (`humans_count`, `PAX_Entry<n>_Req`, `PAX_Exit<n>_Req`).
//!
//! Doors are the scripts' `PAX_Entry<n>_Open` and `PAX_Exit<n>_Open` (the n-th `[entry]` and
//! `[exit]` of the cabin). Specification: `docs/src/simulation/passengers.md`.

use bevy::math::Affine3A;
use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use std::collections::HashMap;

use bevy::prelude::*;
use opts_bevy::bus_stops::{nearest_stop, BusStopMarker};
use opts_bevy::{
    Human, HumanPose, PassengerFeed, PlayerVehicle, VehicleController, VehicleScripts,
};
use opts_sim::passengers::{Doors, Event};

use crate::in_game::omsi_world::{advance_phase, apply_gait, apply_rest_pose, PedestrianPool};

/// Key of the setting that turns the passengers on and off.
pub const SETTING: &str = "gameplay.passengers";
/// The bus counts as being at a stop within this distance of the marker (m).
const AT_STOP_M: f32 = 22.0;
/// …and slower than this (km/h).
const STOPPED_KMH: f32 = 3.0;
/// The bus has left when faster than this (km/h) or farther than [`LEFT_M`] from the stop.
const MOVING_KMH: f32 = 6.0;
/// Distance at which a stop is left (m).
const LEFT_M: f32 = 45.0;
/// Walking speed outside (m/s).
const OUTSIDE_SPEED_M_S: f32 = 1.3;
/// How far a passenger who got off walks away before they vanish (m).
const LEAVE_DISTANCE_M: f32 = 9.0;
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

/// Face `direction` (a horizontal vector) as a rotation about the vertical.
fn facing(direction: Vec3) -> Quat {
    let flat = Vec3::new(direction.x, 0.0, direction.z);
    if flat.length_squared() < 1e-6 {
        Quat::IDENTITY
    } else {
        Transform::IDENTITY.looking_to(flat, Vec3::Y).rotation
    }
}

mod convert;
mod crowd;
mod draw;
mod protocol;
mod rig;
mod weight;
pub(crate) use weight::PassengerCountOverride;

use crowd::{
    stop_key, StopCrowds, ACTOR_BUDGET, DEFAULT_RATE, FIRST_VISIT_WAIT_MIN, MAX_ACCUMULATE_MIN,
    MAX_CROWD, MAX_VISIBLE_WAITING, RATE_SETTING, UNSEEN_BOARD_S,
};
pub use rig::PassengerRig;
use rig::{Shown, Unseen, Walker};

pub use convert::{cabin_spec, paths_spec, ticket_book, unit_spec};
pub use protocol::{command, register};

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
        // Draw the passengers inside.
        draw::draw_inside(rig, &mut commands, &humans, &mut transforms, dt);
        // Those who got off walk away and vanish.
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
                apply_gait(h, w.leg_m, w.phase, &mut transforms);
            }
            walking.push(w);
        }
        rig.leaving = walking;
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
}

/// Registers the passenger system.
pub struct PassengersPlugin;

impl Plugin for PassengersPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (update_passengers, weight::apply_passenger_mass)
                .chain()
                .run_if(in_state(crate::app_state::AppState::InGame)),
        )
        .init_resource::<StopCrowds>()
        .init_resource::<weight::PassengerCountOverride>();
    }
}
