//! The player's collision pass: pushes the player's road vehicles out of static boxes, AI
//! vehicles and ground steps (see the parent module).

use std::collections::HashMap;

use bevy::prelude::*;
use opts_bevy::ArticulatedSegment;
use opts_sim::collision::{respond, Obb};

use super::super::ground_height::GroundIndex;
use super::super::traffic::TrafficVehicle;
use super::{
    obb_of, player_boxes, BodyBox, CollisionIndex, GroundPlayer, PlayerCollide, VehicleCollider,
};

/// Highest obstacle a vehicle drives over: kerbs and road plates (m).
const STEP_M: f32 = 0.3;
/// Largest push out of an overlap in one fixed step (m).
const MAX_PUSH_M: f32 = 0.5;
/// Largest push out of an overlap with an AI vehicle per fixed step (m): the AI is kinematic, so a
/// ram must not throw the player's bus across the road (it used to move it up to 2 m per step).
const MAX_TRAFFIC_PUSH_M: f32 = 0.05;
/// Share of the speed along an obstacle that is lost scraping it.
const FRICTION: f32 = 0.08;
/// Bounce off an obstacle.
const RESTITUTION: f32 = 0.05;

/// Pushes the player's vehicle out of everything it overlaps (fixed step, after the physics).
#[allow(clippy::too_many_arguments)]
pub fn player_collisions(
    mut logged: Local<u32>,
    index: Res<CollisionIndex>,
    ground: Res<GroundIndex>,
    mut players: Query<(Entity, PlayerCollide), GroundPlayer>,
    mut embedded: Local<HashMap<Entity, Vec<Obb>>>,
    segments: Query<&ArticulatedSegment>,
    traffic: Query<(&GlobalTransform, &VehicleCollider), With<TrafficVehicle>>,
) {
    if std::env::var_os("OPTS_DBG_COLLISION").is_some() {
        tracing::info!(static_boxes = index.len(), "collision index");
    }
    embedded.retain(|e, _| players.contains(*e));
    for (entity, (mut controller, head, chain, composition)) in &mut players {
        // Static boxes the vehicle was placed inside (a spawn point in a bus stop's collision
        // box) do not push it: it is on its way out when it drives off. A box it no longer
        // touches is forgotten, so it is solid again afterwards.
        let first_seen = !embedded.contains_key(&entity);
        let free = embedded.entry(entity).or_default();
        let mut touching: Vec<Obb> = Vec::new();
        // The following parts' boxes: from their parts' `.bus` boxes, at their simulated poses.
        let part_boxes: Vec<BodyBox> = composition
            .map(|c| c.followers.iter().filter_map(|p| p.body).collect())
            .unwrap_or_default();
        let dynamic: Vec<Obb> = traffic
            .iter()
            .map(|(g, c)| obb_of(g, c.centre, c.half))
            .collect();
        let mut push_total = Vec2::ZERO;
        let mut normals = Vec::new();
        for _ in 0..4 {
            let ch = &controller.sim.chassis;
            let followers: Vec<(BodyBox, Vec2, f32)> = chain
                .map(|c| {
                    c.segments
                        .iter()
                        .zip(&part_boxes)
                        .filter_map(|(e, b)| {
                            let (p, h) = segments.get(*e).ok()?.simulated_pose()?;
                            Some((*b, p, h))
                        })
                        .collect()
                })
                .unwrap_or_default();
            let mine = player_boxes(
                ch.position,
                ch.heading_rad,
                ch.elevation_z,
                head,
                &followers,
            );
            let mut worst: Option<(Vec2, f32)> = None;
            let mut worst_is_traffic = false;
            let mut worst_other: Option<Obb> = None;
            for me in &mine {
                let statics = index.grid.query(me.centre, me.radius()).map(|o| (o, false));
                for (other, is_traffic) in statics.chain(dynamic.iter().map(|o| (o, true))) {
                    if !me.overlaps_height(other, STEP_M) {
                        continue;
                    }
                    if let Some(c) = me.contact(other) {
                        if !is_traffic {
                            if first_seen && !free.contains(other) {
                                free.push(*other);
                            }
                            if free.contains(other) {
                                if !touching.contains(other) {
                                    touching.push(*other);
                                }
                                continue;
                            }
                        }
                        if worst.is_none_or(|w| c.depth > w.1) {
                            worst = Some((c.normal, c.depth));
                            worst_is_traffic = is_traffic;
                            worst_other = Some(*other);
                        }
                    }
                }
            }
            // Steps in the ground around the head's footprint.
            if let Some(n) = ground_step(&ground, &mine[0], ch.elevation_z, head.0.height) {
                if worst.is_none_or(|w| 0.05 > w.1) {
                    worst = Some((n, 0.05));
                }
            }
            let Some((normal, depth)) = worst else { break };
            if !worst_is_traffic
                && *logged < 400
                && std::env::var_os("OPTS_DBG_COLLISION").is_some()
            {
                *logged += 1;
                tracing::info!(
                    at = ?controller.sim.chassis.position,
                    z = controller.sim.chassis.elevation_z,
                    other = ?worst_other,
                    depth,
                    "the player's vehicle is pushed by a static box or a ground step"
                );
            }
            if worst_is_traffic && *logged < 40 {
                *logged += 1;
                tracing::info!(
                    at = ?controller.sim.chassis.position,
                    speed = controller.sim.chassis.forward_speed_m_s(),
                    other = ?worst_other,
                    depth,
                    "the player's vehicle overlaps an AI vehicle and is pushed"
                );
            }
            let cap = if worst_is_traffic {
                MAX_TRAFFIC_PUSH_M
            } else {
                MAX_PUSH_M
            };
            let push = (depth + 0.01).min(cap - push_total.length()).max(0.0);
            if push <= 0.0 {
                break;
            }
            controller.sim.chassis.position += normal * push;
            push_total += normal * push;
            normals.push(normal);
        }
        free.retain(|b| touching.contains(b));
        if normals.is_empty() {
            continue;
        }
        // Velocity: nothing into the obstacle, a little scraped off along it.
        let ch = &mut controller.sim.chassis;
        let mut v = ch.world_velocity();
        for n in &normals {
            v = respond(v, *n, RESTITUTION, FRICTION);
        }
        let (s, c) = ch.heading_rad.sin_cos();
        ch.local_velocity = Vec2::new(v.x * c - v.y * s, v.x * s + v.y * c);
        ch.yaw_rate_rad_s *= 0.7;
    }
}

/// Direction away from a step in the ground around a footprint, if there is one: points on the
/// outline are compared with the ground 0.4 m closer to the centre. A ground higher than
/// `clearance` above the vehicle (the deck of a bridge over the road) is no step.
fn ground_step(ground: &GroundIndex, body: &Obb, z: f32, clearance: f32) -> Option<Vec2> {
    let [fwd, right] = body.axes();
    let mut away = Vec2::ZERO;
    for (a, b) in [(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)] {
        for k in [-0.8_f32, 0.0, 0.8] {
            let (lx, ly) = if a != 0.0 {
                (a * body.half.x, k * body.half.y)
            } else {
                (k * body.half.x, b * body.half.y)
            };
            let edge = body.centre + right * lx + fwd * ly;
            let inward = (body.centre - edge).normalize_or_zero();
            let (Some(outer), Some(inner)) = (
                ground.height_at(edge.x + (-inward.x) * 0.1, edge.y + (-inward.y) * 0.1, z),
                ground.height_at(edge.x + inward.x * 0.3, edge.y + inward.y * 0.3, z),
            ) else {
                continue;
            };
            // A surface higher than the vehicle is a deck it passes under, not a step.
            if outer - inner > STEP_M && outer > z + STEP_M && outer < z + clearance {
                away += inward;
            }
        }
    }
    (away.length_squared() > 0.0).then(|| away.normalize())
}
