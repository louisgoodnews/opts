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

use std::collections::HashMap;

use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy::prelude::*;
use opts_bevy::bus_stops::{nearest_stop, BusStopMarker};
use opts_bevy::coords::omsi_to_bevy_pos;
use opts_bevy::{
    Human, HumanPose, PassengerFeed, PlayerVehicle, VehicleController, VehicleScripts,
};
use opts_core::formats::ticketpack::TicketPack;
use opts_core::formats::vehicles::cabin::PassengerCabin;
use opts_core::formats::vehicles::PassengerPaths;
use opts_sim::passengers::{Doors, Event, PassengerSim, Phase, Rng, WALK_SPEED_M_S};

use crate::in_game::omsi_world::{advance_phase, apply_gait, apply_rest_pose, PedestrianPool};

/// The bus counts as being at a stop within this distance of the marker (m).
const AT_STOP_M: f32 = 22.0;
/// …and slower than this (km/h).
const STOPPED_KMH: f32 = 3.0;
/// The bus has left when faster than this (km/h) or farther than [`LEFT_M`] from the stop.
const MOVING_KMH: f32 = 6.0;
/// Distance at which a stop is left (m).
const LEFT_M: f32 = 45.0;
/// How far from the door the people come to wait for it to open (m).
const DOOR_APPROACH_M: f32 = 0.9;
/// Walking speed outside (m/s).
const OUTSIDE_SPEED_M_S: f32 = 1.3;
/// How far a passenger who got off walks away before they vanish (m).
const LEAVE_DISTANCE_M: f32 = 9.0;
/// Most people waiting at a stop.
const MAX_WAITING: u32 = 7;

/// The passengers of a vehicle.
#[derive(Component)]
pub struct PassengerRig {
    /// The simulation of the main unit.
    pub sim: PassengerSim,
    stop: Option<Entity>,
    actors: HashMap<u32, Shown>,
    approaching: Vec<Walker>,
    leaving: Vec<Walker>,
    rng: Rng,
    /// Visits of stops so far (for the sounds and the report).
    pub stops_served: u32,
    /// People to send to the doors now, stop or no stop (`vehicle.passengers spawn`).
    pub force_group: u32,
}

/// A passenger inside, as drawn.
struct Shown {
    entity: Entity,
    leg_m: f32,
    phase: f32,
    local: Vec3,
    seated: Option<f32>,
}

/// A person outside: walking up to the bus or away from it.
struct Walker {
    entity: Entity,
    leg_m: f32,
    phase: f32,
    position: Vec3,
    target: Vec3,
    entry: usize,
    age: u32,
    stops: u32,
    walked_m: f32,
}

impl PassengerRig {
    /// A rig from the files of a unit, `None` when the cabin has no entry or no pathpoints.
    #[must_use]
    pub fn from_files(cabin: &[u8], paths: &[u8], tickets: TicketPack, seed: u64) -> Option<Self> {
        let cabin = PassengerCabin::parse_bytes(cabin);
        let paths = PassengerPaths::parse_bytes(paths).ok()?;
        if cabin.entries.is_empty() || paths.points.is_empty() {
            return None;
        }
        Some(Self {
            sim: PassengerSim::new(
                cabin_spec(&cabin),
                &paths_spec(&paths),
                ticket_book(&tickets),
                seed,
            ),
            stop: None,
            actors: HashMap::new(),
            approaching: Vec::new(),
            leaving: Vec::new(),
            rng: Rng::new(seed ^ 0xA5A5),
            stops_served: 0,
            force_group: 0,
        })
    }

    /// The point in front of entry `entry` where people wait (vehicle-local Bevy axes).
    fn door_outside(&self, entry: usize) -> Vec3 {
        let node = self.sim.cabin.entries.get(entry).copied().unwrap_or(0);
        let p = self
            .sim
            .graph()
            .points
            .get(node)
            .copied()
            .unwrap_or(Vec3::ZERO);
        let out = if p.x >= 0.0 { 1.0 } else { -1.0 };
        omsi_to_bevy_pos(p + Vec3::new(out * DOOR_APPROACH_M, 0.0, 0.0))
    }

    /// The entry nearest to the point `local` (vehicle-local Bevy axes).
    fn nearest_entry(&self, local: Vec3) -> usize {
        (0..self.sim.cabin.entries.len())
            .min_by(|&a, &b| {
                self.door_outside(a)
                    .distance(local)
                    .total_cmp(&self.door_outside(b).distance(local))
            })
            .unwrap_or(0)
    }
}

/// The doors the vehicle's scripts report.
fn read_doors(scripts: &VehicleScripts, entries: usize, exits: usize) -> Doors {
    let Ok(engine) = scripts.engine.lock() else {
        return Doors::default();
    };
    let open = |kind: &str, n: usize| engine.get(&format!("pax_{kind}{n}_open")) > 0.5;
    Doors {
        entries: (0..entries).map(|n| open("entry", n)).collect(),
        exits: (0..exits).map(|n| open("exit", n)).collect(),
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
mod protocol;

pub use convert::{cabin_spec, paths_spec, ticket_book};
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
            &VehicleScripts,
        ),
        With<PlayerVehicle>,
    >,
    stops: Query<(Entity, &GlobalTransform), With<BusStopMarker>>,
    humans: Query<&Human>,
    mut transforms: Query<&mut Transform>,
) {
    let dt = time.delta_secs().min(0.1);
    if dt <= 0.0 {
        return;
    }
    for (vehicle, mut rig, vgt, controller, scripts) in &mut rigs {
        pool.want();
        let rig = &mut *rig;
        let to_world = vgt.affine();
        let to_local = to_world.inverse();
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
            let door = rig.door_outside(0);
            let side = if door.x >= 0.0 { 1.0 } else { -1.0 };
            for slot in 0..n {
                let Some(actor) =
                    pool.spawn_actor(&mut commands, &mut bindposes, rig.rng.next_f32())
                else {
                    break;
                };
                let local = door
                    + Vec3::new(
                        side * (1.4 + (slot % 3) as f32 * 0.5),
                        0.0,
                        (slot as f32 - n as f32 / 2.0) * 0.7,
                    );
                let spot = to_world.transform_point3(local);
                let entry = rig.nearest_entry(local);
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
                    let n = (rig.rng.next_f32() * (MAX_WAITING + 1) as f32) as u32;
                    for slot in 0..n {
                        let spot = BusStopMarker::waiting_area(&tf, slot);
                        let Some(actor) =
                            pool.spawn_actor(&mut commands, &mut bindposes, rig.rng.next_f32())
                        else {
                            break;
                        };
                        let local = to_local.transform_point3(spot);
                        let entry = rig.nearest_entry(local);
                        let age = actor.age.unwrap_or(18 + (rig.rng.next_f32() * 50.0) as u32);
                        let stops_to_ride = 1 + (rig.rng.next_f32() * 6.0) as u32;
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
                    for w in rig.approaching.drain(..) {
                        commands.entity(w.entity).despawn();
                    }
                }
            }
            _ => {}
        }
        // People walking up to the doors; they board when the door is open.
        let doors = read_doors(
            scripts,
            rig.sim.cabin.entries.len(),
            rig.sim.cabin.exits.len(),
        );
        let mut still = Vec::new();
        for mut w in std::mem::take(&mut rig.approaching) {
            w.target = to_world.transform_point3(rig.door_outside(w.entry));
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
            } else if doors.entries.get(w.entry).copied().unwrap_or(false) {
                if let Some(id) = rig.sim.board(w.age, w.stops, w.entry) {
                    let local = to_local.transform_point3(w.position);
                    commands.entity(w.entity).insert(ChildOf(vehicle));
                    if let Ok(mut t) = transforms.get_mut(w.entity) {
                        t.translation = local;
                    }
                    rig.actors.insert(
                        id,
                        Shown {
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
        // The passengers.
        rig.sim.step(dt, &doors);
        for e in rig.sim.take_events() {
            match e {
                Event::Settled { id, place, seated } => {
                    if let Some(s) = rig.actors.get_mut(&id) {
                        s.seated =
                            seated.then(|| rig.sim.cabin.places[place].rotation_deg.to_radians());
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
                        let world = to_world.transform_point3(s.local);
                        let away = to_world
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
        for p in rig.sim.passengers() {
            let Some(s) = rig.actors.get_mut(&p.id) else {
                continue;
            };
            let target = omsi_to_bevy_pos(p.position);
            let moving = matches!(
                p.phase,
                Phase::Walking { .. } | Phase::WaitingOutside { .. }
            );
            let delta = target - s.local;
            let d = delta.length();
            let step = WALK_SPEED_M_S * 1.5 * dt;
            let yaw = if d > 0.02 { Some(facing(delta)) } else { None };
            s.local = if d <= step {
                target
            } else {
                s.local + delta / d * step
            };
            let Ok(mut t) = transforms.get_mut(s.entity) else {
                continue;
            };
            t.translation = s.local;
            if let Some(q) = yaw.filter(|_| moving || d > 0.05) {
                t.rotation = q;
            } else if let Some(rot) = s.seated {
                t.rotation = Quat::from_rotation_y(-rot);
            }
            if let Ok(h) = humans.get(s.entity) {
                if d > 0.02 {
                    s.phase = advance_phase(h, s.phase, WALK_SPEED_M_S, dt);
                    apply_gait(h, s.leg_m, s.phase, &mut transforms);
                } else {
                    let pose = if s.seated.is_some() {
                        HumanPose::Seated
                    } else {
                        HumanPose::Standing
                    };
                    apply_rest_pose(h, pose, &mut transforms);
                }
            }
        }
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
        // What the scripts see.
        let mut feed = PassengerFeed {
            humans: rig.sim.humans_inside() as u32,
            ..default()
        };
        for (i, r) in rig
            .sim
            .entry_requests()
            .into_iter()
            .enumerate()
            .take(feed.entry_req.len())
        {
            feed.entry_req[i] = r
                || rig
                    .approaching
                    .iter()
                    .any(|w| w.entry == i && w.position.distance(w.target) <= 0.4);
        }
        for (i, r) in rig
            .sim
            .exit_requests()
            .into_iter()
            .enumerate()
            .take(feed.exit_req.len())
        {
            feed.exit_req[i] = r;
        }
        commands.entity(vehicle).insert(feed);
    }
}

/// Registers the passenger system.
pub struct PassengersPlugin;

impl Plugin for PassengersPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            update_passengers.run_if(in_state(crate::app_state::AppState::InGame)),
        );
    }
}
