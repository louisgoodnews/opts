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

mod board;
mod doors;
mod walk;
use board::{board_unseen, send_everybody_home};
use doors::read_doors;
use walk::walk_to_doors;


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
