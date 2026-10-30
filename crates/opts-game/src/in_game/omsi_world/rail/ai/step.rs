//! Stepping AI trains and cyclists along the track network and posing their coupled units.

use super::*;

/// What the player's vehicle contributes to the obstacle list.
type PlayerFootprint = (
    &'static GlobalTransform,
    Option<&'static crate::in_game::omsi_world::PlayerBox>,
    Option<&'static VehicleComposition>,
);

/// Gap from the front of the train's head to the nearest obstacle ahead in its lane.
fn ahead_gap(
    t: &AiTrain,
    net: &PedestrianNet,
    owner: Entity,
    obstacles: &[Obstacle],
) -> Option<f32> {
    let (p, h) = t.train.pose(net)?;
    let here = omsi_to_bevy_pos(p);
    let ahead = omsi_to_bevy_pos(p + Vec3::new(h.sin(), h.cos(), 0.0));
    let dir = Vec2::new(ahead.x - here.x, ahead.z - here.z).normalize_or_zero();
    if dir == Vec2::ZERO {
        return None;
    }
    // From the head unit's front end (its footprint's far end; 3 m when unknown).
    let front = t
        .composition
        .head
        .body
        .map_or(3.0, |b| b.offset.y.abs() + b.half.y);
    let origin = Vec2::new(here.x, here.z) + dir * front;
    nearest_gap(obstacles, owner, origin, dir, 60.0)
}


/// What a train must not run into: every unit of the player's traction (its last unit's rear
/// counts, not just the head) and of the other trains.
fn collect_obstacles(
    players: &Query<PlayerFootprint, With<opts_bevy::PlayerVehicle>>,
    globals: &Query<&GlobalTransform>,
    trains: &Query<(Entity, &mut AiTrain)>,
    units: &Query<(&mut Transform, &mut Visibility, &mut opts_bevy::AiVariables)>,
) -> Vec<Obstacle> {
    let mut obstacles: Vec<Obstacle> = Vec::new();
    for (g, body, composition) in &players {
        let at = g.compute_transform();
        obstacles.push(Obstacle::new(Entity::PLACEHOLDER, &at, body.map(|b| b.0)));
        for p in composition.iter().flat_map(|c| c.followers.iter()) {
            if let Ok(g) = globals.get(p.entity) {
                obstacles.push(Obstacle::new(
                    Entity::PLACEHOLDER,
                    &g.compute_transform(),
                    p.body,
                ));
            }
        }
    }
    for (owner, t) in &trains {
        for p in t.composition.parts() {
            if let Ok((tf, ..)) = units.get(p.entity) {
                obstacles.push(Obstacle::new(owner, tf, p.body));
            }
        }
    }
    obstacles
}

/// Puts every unit of a train on the track at its offset, shows it and turns its wheels.
fn place_units(
    t: &AiTrain,
    net: &RailNet,
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

/// Runs the AI trains along their track and poses every unit by its distance behind the head.
#[allow(clippy::too_many_arguments)]
pub fn move_ai_trains(
    time: Res<Time>,
    mut commands: Commands,
    rail_net: Res<RailNetwork>,
    bike_net: Res<BikeNetwork>,
    camera: Query<&GlobalTransform, With<opts_bevy::MainCamera>>,
    players: Query<PlayerFootprint, With<opts_bevy::PlayerVehicle>>,
    globals: Query<&GlobalTransform>,
    mut trains: Query<(Entity, &mut AiTrain)>,
    mut units: Query<(&mut Transform, &mut Visibility, &mut opts_bevy::AiVariables)>,
    env: Option<Res<opts_bevy::SessionEnvironment>>,
    mut sched: ResMut<crate::in_game::omsi_world::ScheduledTraffic>,
) {
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
