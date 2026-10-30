//! Main update system for passengers of the player's vehicle.

use bevy::math::Affine3A;
use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy::prelude::*;
use opts_bevy::bus_stops::{nearest_stop, BusStopMarker};
use opts_bevy::{Human, HumanPose, PlayerVehicle, VehicleController, VehicleScripts};
use opts_sim::passengers::Doors;

use crate::in_game::omsi_world::{advance_phase, apply_gait, apply_rest_pose, PedestrianPool};

use super::crowd::{StopCrowds, ACTOR_BUDGET, UNSEEN_BOARD_S};
use super::rig::{PassengerRig, Shown};
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

/// The queries and resources the passengers' systems share.
#[derive(bevy::ecs::system::SystemParam)]
pub struct PassengerEnv<'w, 's> {
    commands: Commands<'w, 's>,
    pool: ResMut<'w, PedestrianPool>,
    bindposes: ResMut<'w, Assets<SkinnedMeshInverseBindposes>>,
    engines: Query<'w, 's, &'static VehicleScripts>,
    stops: Query<'w, 's, (Entity, &'static GlobalTransform), With<BusStopMarker>>,
    humans: Query<'w, 's, &'static Human>,
    globals: Query<'w, 's, &'static GlobalTransform>,
    transforms: Query<'w, 's, &'static mut Transform>,
    crowds: ResMut<'w, StopCrowds>,
}

mod stop;

use stop::{arrive_at_stop, leave_stop, spawn_forced_group};

/// Switched off: everybody goes at once.
fn send_everybody_home(e: &mut PassengerEnv, rig: &mut PassengerRig) {
    for (_, s) in rig.actors.drain() {
        e.commands.entity(s.entity).despawn();
    }
    for w in rig.approaching.drain(..).chain(rig.leaving.drain(..)) {
        e.commands.entity(w.entity).despawn();
    }
    rig.sim.clear();
    rig.stop = None;
}

/// People walking up to the doors; they board when the door is open.
fn walk_to_doors(
    e: &mut PassengerEnv,
    rig: &mut PassengerRig,
    (frames, to_world, vehicle): (&[Affine3A], Affine3A, Entity),
    doors: &Doors,
    dt: f32,
) {
    let mut still = Vec::new();
    for mut w in std::mem::take(&mut rig.approaching) {
        w.target = rig.door_world(w.entry, frames);
        let d = w.position.distance(w.target);
        let step = OUTSIDE_SPEED_M_S * dt;
        if d > 0.25 {
            let dir = (w.target - w.position) / d;
            w.position += dir * step.min(d);
            if let Ok(mut t) = e.transforms.get_mut(w.entity) {
                t.translation = w.position;
                t.rotation = facing(dir);
            }
            if let Ok(h) = e.humans.get(w.entity) {
                w.phase = advance_phase(h, w.phase, OUTSIDE_SPEED_M_S, dt);
                apply_gait(h, w.leg_m, w.phase, &mut e.transforms);
            }
            still.push(w);
        } else if doors.entries.get(w.entry).copied().unwrap_or(false)
            && rig.sim.passengers().len() < rig.sim.capacity()
        {
            if let Some(id) = rig.sim.board(w.age, w.stops, w.entry) {
                // Beyond the figure budget a person boards unseen.
                if rig.actors.len() >= ACTOR_BUDGET {
                    e.commands.entity(w.entity).despawn();
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
                e.commands.entity(w.entity).insert(ChildOf(parent));
                if let Ok(mut t) = e.transforms.get_mut(w.entity) {
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
                e.commands.entity(w.entity).despawn();
            }
        } else {
            // Waiting for the door: stand still.
            if let Ok(h) = e.humans.get(w.entity) {
                apply_rest_pose(h, HumanPose::Standing, &mut e.transforms);
            }
            still.push(w);
        }
    }
    rig.approaching = still;
}

/// The rest of the crowd boards through the open doors, one person per door in turn, while there
/// is room.
fn board_unseen(rig: &mut PassengerRig, doors: &Doors, dt: f32) {
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
}

/// What a vehicle's passengers are driven from.
type RigRow = (
    Entity,
    &'static mut PassengerRig,
    &'static GlobalTransform,
    &'static VehicleController,
    Option<&'static opts_bevy::host_callbacks::PassengerCountsHandle>,
);

/// Runs the passengers of the player's vehicle.
pub fn update_passengers(
    time: Res<Time>,
    mut rigs: Query<RigRow, With<PlayerVehicle>>,
    mut e: PassengerEnv,
    settings: Option<Res<crate::settings::GameSettings>>,
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
            send_everybody_home(&mut e, rig);
            continue;
        }
        e.pool.want();
        let to_world = vgt.affine();
        let frames: Vec<Affine3A> = rig
            .units
            .iter()
            .map(|u| {
                e.globals
                    .get(u.root)
                    .map_or(to_world, GlobalTransform::affine)
            })
            .collect();
        let speed = controller.forward_speed_km_h().abs();
        let here = vgt.translation();
        let markers: Vec<(Entity, Transform)> = e
            .stops
            .iter()
            .map(|(m, g)| (m, Transform::from_translation(g.translation())))
            .collect();
        let nearest = nearest_stop(markers.iter().map(|(m, t)| (*m, t)), here, LEFT_M);
        if rig.force_group > 0 {
            spawn_forced_group(&mut e, rig, &frames, to_world);
        }
        // Arriving at and leaving a stop.
        match (rig.stop, nearest) {
            (None, Some((marker, d))) if d <= AT_STOP_M && speed <= STOPPED_KMH => {
                arrive_at_stop(
                    &mut e,
                    rig,
                    (marker, time.elapsed_secs_f64()),
                    &frames,
                    (settings.as_deref(), job.as_deref(), env.as_deref()),
                );
            }
            (Some(_), near) => {
                let gone = near.is_none_or(|(_, d)| d > LEFT_M) || speed > MOVING_KMH;
                if gone {
                    leave_stop(&mut e, rig);
                }
            }
            _ => {}
        }
        let (entry_slots, exit_slots) = rig.door_slots();
        let doors = read_doors(&e.engines, &entry_slots, &exit_slots);
        walk_to_doors(&mut e, rig, (&frames, to_world, vehicle), &doors, dt);
        board_unseen(rig, &doors, dt);
        // The passengers.
        rig.sim.step(dt, &doors);
        feed::apply_sim_events(rig, &frames, to_world, &mut e.commands, &mut e.transforms);
        // Draw the passengers inside.
        draw::draw_inside(rig, &mut e.commands, &e.humans, &mut e.transforms, dt);
        // Those who got off walk away and vanish.
        feed::step_leaving(rig, &mut e.commands, &e.humans, &mut e.transforms, dt);
        feed::publish_feeds(
            vehicle,
            rig,
            handle,
            &entry_slots,
            &exit_slots,
            &mut e.commands,
        );
    }
}
