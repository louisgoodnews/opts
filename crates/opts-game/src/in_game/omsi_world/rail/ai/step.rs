//! Stepping AI trains and cyclists along the track network and posing their coupled units.

use super::*;

mod obstacles;
use obstacles::{ahead_gap, collect_obstacles, PlayerFootprint};

/// Puts every unit of a train on the track at its offset, shows it and turns its wheels.
fn place_units(
    t: &AiTrain,
    net: &PedestrianNet,
    units: &mut Query<(&mut Transform, &mut Visibility, &mut opts_bevy::AiVariables)>,
) {
    let angle = t.wheel_angle;
    let head_span = t.composition.head.half_span;
    let list: Vec<Part> = t.composition.parts().cloned().collect();
    for part in list {
        let (e, reversed) = (part.entity, part.reversed);
        let Some(pose) = unit_pose(
            &t.train,
            net,
            part.behind + head_span,
            part.half_span,
            reversed,
            0.0,
        ) else {
            continue;
        };
        if let Ok((mut tf, mut vis, mut vars)) = units.get_mut(e) {
            *tf = pose;
            *vis = Visibility::Visible;
            for axle in 0..6 {
                for side in ["l", "r"] {
                    vars.set(&format!("wheel_rotation_{axle}_{side}"), angle);
                }
            }
        }
    }
}

/// The networks, clock and schedule the AI trains move by.
#[derive(bevy::ecs::system::SystemParam)]
pub struct AiMoveWorld<'w> {
    rail_net: Res<'w, RailNetwork>,
    bike_net: Res<'w, BikeNetwork>,
    env: Option<Res<'w, opts_bevy::SessionEnvironment>>,
    sched: ResMut<'w, crate::in_game::omsi_world::ScheduledTraffic>,
}

/// The queries around the AI trains: the camera, the player, and the units to pose.
#[derive(bevy::ecs::system::SystemParam)]
pub struct AiMoveViews<'w, 's> {
    camera: Query<'w, 's, &'static GlobalTransform, With<opts_bevy::MainCamera>>,
    players: Query<'w, 's, PlayerFootprint, With<opts_bevy::PlayerVehicle>>,
    globals: Query<'w, 's, &'static GlobalTransform>,
    units: Query<
        'w,
        's,
        (
            &'static mut Transform,
            &'static mut Visibility,
            &'static mut opts_bevy::AiVariables,
        ),
    >,
}

/// Runs the AI trains along their track and poses every unit by its distance behind the head.
pub fn move_ai_trains(
    time: Res<Time>,
    mut commands: Commands,
    world: AiMoveWorld,
    views: AiMoveViews,
    mut trains: Query<(Entity, &mut AiTrain)>,
) {
    let AiMoveWorld {
        rail_net,
        bike_net,
        env,
        mut sched,
    } = world;
    let AiMoveViews {
        camera,
        players,
        globals,
        mut units,
    } = views;
    let clock = env.as_ref().map_or(0.0, |e| e.seconds_of_day());
    let dt = time.delta_secs().min(0.1);
    let cam = camera.single().ok().map(|c| c.translation());
    let obstacles = collect_obstacles(&players, &globals, &trains, &units);
    for (head, mut t) in &mut trains {
        let net = if t.bike { &bike_net.0 } else { &rail_net.0 };
        let (accel, cruise) = if t.bike {
            (1.0, BIKE_CRUISE_MS)
        } else {
            (0.8, AI_CRUISE_MS)
        };
        t.speed = (t.speed + accel * dt).min(cruise);
        if let Some(gap) = ahead_gap(&t, net, head, &obstacles) {
            // Stop 4 m short of it with a comfortable 1.5 m/s² (the lane clears when it moves on).
            t.speed = t.speed.min((2.0 * 1.5 * (gap - 4.0).max(0.0)).sqrt());
        }
        if let Some(limit) = super::super::scheduled::control(&mut t, net, clock, &mut sched) {
            t.speed = t.speed.min(limit);
        }
        let step = t.speed * dt;
        let end = t.train.advance(step, net) == Moved::EndOfTrack;
        let far = cam.is_some_and(|c| {
            t.train
                .pose(net)
                .is_some_and(|(p, _)| c.distance(omsi_to_bevy_pos(p)) > AI_DESPAWN_M)
        });
        if end
            || far
            || t.train.pose(net).is_none()
            || super::super::scheduled::retired(&t, net, clock, cam)
        {
            if let Some(run) = super::super::scheduled::run_of(&t) {
                sched.rail_done(run);
            }
            for p in t.composition.parts() {
                commands.entity(p.entity).despawn();
            }
            let _ = head;
            continue;
        }
        t.wheel_angle += step / 0.29;
        place_units(&t, net, &mut units);
    }
}
