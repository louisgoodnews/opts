//! What the AI trains must not run into: the obstacle list of the player's and the other trains'
//! units, and the gap ahead of a train in its lane.

use super::*;

/// What the player's vehicle contributes to the obstacle list.
pub(super) type PlayerFootprint = (
    &'static GlobalTransform,
    Option<&'static crate::PlayerBox>,
    Option<&'static VehicleComposition>,
);

/// Gap from the front of the train's head to the nearest obstacle ahead in its lane.
pub(super) fn ahead_gap(
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
pub(super) fn collect_obstacles(
    players: &Query<PlayerFootprint, With<opts_bevy::PlayerVehicle>>,
    globals: &Query<&GlobalTransform>,
    trains: &Query<(Entity, &mut AiTrain)>,
    units: &Query<(&mut Transform, &mut Visibility, &mut opts_bevy::AiVariables)>,
) -> Vec<Obstacle> {
    let mut obstacles: Vec<Obstacle> = Vec::new();
    for (g, body, composition) in players {
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
    for (owner, t) in trains {
        for p in t.composition.parts() {
            if let Ok((tf, ..)) = units.get(p.entity) {
                obstacles.push(Obstacle::new(owner, tf, p.body));
            }
        }
    }
    obstacles
}
