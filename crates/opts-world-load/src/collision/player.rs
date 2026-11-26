//! The player's collision pass: pushes the player's road vehicles out of static boxes, AI
//! vehicles and ground steps (see the parent module). The contact search is in [`contact`], the
//! helpers of the pass in [`support`].

use std::collections::HashMap;

use bevy::prelude::*;
use opts_bevy::ArticulatedSegment;
use opts_collision::Obb;

use super::super::ground_height::GroundIndex;
use super::super::traffic::TrafficVehicle;
use super::{
    obb_of, player_boxes, BodyBox, CollisionIndex, GroundPlayer, PlayerCollide, VehicleCollider,
};

mod chain;
mod contact;
mod support;

use chain::push_part;
use contact::{ground_step, worst_contact, Worst};
use support::{follower_poses, log_contact, log_push, scrape_velocity};

/// Highest obstacle a vehicle drives over: kerbs and road plates (m).
const STEP_M: f32 = 0.3;
/// Pushes shorter than this (m) are not passed on along the chain.
const SPREAD_MIN_M: f64 = 1e-4;
/// Largest push out of an overlap in one fixed step (m).
const MAX_PUSH_M: f32 = 0.5;
/// Largest push out of an overlap with an AI vehicle per fixed step (m): the AI is kinematic, so a
/// ram must not throw the player's bus across the road (it used to move it up to 2 m per step).
const MAX_TRAFFIC_PUSH_M: f32 = 0.05;
/// Share of the speed along an obstacle that is lost scraping it.
const FRICTION: f32 = 0.08;
/// Bounce off an obstacle.
const RESTITUTION: f32 = 0.05;

/// The AI vehicles' boxes at their current poses, for the pass.
fn traffic_boxes(
    traffic: &Query<(&GlobalTransform, &VehicleCollider), With<TrafficVehicle>>,
) -> Vec<Obb> {
    traffic
        .iter()
        .map(|(g, c)| obb_of(g, c.centre, c.half))
        .collect()
}

/// The length of the push out of an overlap this step: the depth plus a margin, limited by the
/// cap of the obstacle's kind (AI vehicles cap lower) less what was already pushed, never negative.
fn push_amount(push_total: Vec2, depth: f32, is_traffic: bool) -> f32 {
    let cap = if is_traffic {
        MAX_TRAFFIC_PUSH_M
    } else {
        MAX_PUSH_M
    };
    (depth + 0.01).min(cap - push_total.length()).max(0.0)
}

/// Pushes the player's vehicle out of everything it overlaps (fixed step, after the physics).
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
        let dynamic: Vec<Obb> = traffic_boxes(&traffic);
        let mut push_total = Vec2::ZERO;
        let mut normals = Vec::new();
        for _ in 0..4 {
            let ch = &controller.sim.chassis;
            let followers = follower_poses(&controller.sim, chain, &part_boxes, &segments);
            let mine = player_boxes(
                ch.position,
                ch.heading_rad,
                ch.elevation_z,
                head,
                &followers,
            );
            let Worst {
                mut contact,
                traffic: worst_is_traffic,
                other: worst_other,
                part: worst_part,
            } = worst_contact(&mine, (&index, &dynamic), (first_seen, free, &mut touching));
            let mut hit = worst_part;
            // Steps in the ground around the head's footprint.
            if let Some(n) = ground_step(&ground, &mine[0], ch.elevation_z, head.0.height) {
                if contact.is_none_or(|w| 0.05 > w.1) {
                    contact = Some((n, 0.05));
                    hit = 0;
                }
            }
            let Some((normal, depth)) = contact else {
                break;
            };
            log_push(
                &mut logged,
                &controller,
                worst_is_traffic,
                worst_other,
                depth,
            );
            let push = push_amount(push_total, depth, worst_is_traffic);
            if push <= 0.0 {
                break;
            }
            log_contact(
                hit,
                mine[hit],
                worst_other,
                normal,
                depth,
                push,
                worst_is_traffic,
            );
            push_part(&mut controller.sim, hit, (normal * push).as_dvec2());
            push_total += normal * push;
            normals.push(normal);
        }
        free.retain(|b| touching.contains(b));
        if normals.is_empty() {
            continue;
        }
        scrape_velocity(&mut controller.sim.chassis, &normals);
    }
}
