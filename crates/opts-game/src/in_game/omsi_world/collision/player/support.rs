//! Helpers of the collision pass: the following parts' poses, the log of a push and the scrape of
//! the velocity along the obstacles.

use bevy::prelude::*;
use opts_bevy::ArticulatedSegment;
use opts_collision::{respond, Obb};

use super::super::BodyBox;
use super::{FRICTION, RESTITUTION};

/// The following parts' simulated poses with their boxes: (box, position, heading).
pub(super) fn follower_poses(
    chain: Option<&opts_bevy::ArticulatedChain>,
    part_boxes: &[BodyBox],
    segments: &Query<&ArticulatedSegment>,
) -> Vec<(BodyBox, Vec2, f32)> {
    chain
        .map(|c| {
            c.segments
                .iter()
                .zip(part_boxes)
                .filter_map(|(e, b)| {
                    let (p, h) = segments.get(*e).ok()?.simulated_pose()?;
                    Some((*b, p, h))
                })
                .collect()
        })
        .unwrap_or_default()
}

/// `OPTS_DBG_COLLISION` and the first pushes by AI vehicles: what pushed the player's vehicle.
pub(super) fn log_push(
    logged: &mut u32,
    controller: &opts_bevy::VehicleController,
    worst_is_traffic: bool,
    worst_other: Option<Obb>,
    depth: f32,
) {
    if !worst_is_traffic && *logged < 400 && std::env::var_os("OPTS_DBG_COLLISION").is_some() {
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
}

/// Velocity: nothing into the obstacle, a little scraped off along it.
pub(super) fn scrape_velocity(ch: &mut opts_sim::vehicle::chassis::ChassisState, normals: &[Vec2]) {
    let mut v = ch.world_velocity();
    for n in normals {
        v = respond(v, *n, RESTITUTION, FRICTION);
    }
    let (s, c) = ch.heading_rad.sin_cos();
    ch.local_velocity = Vec2::new(v.x * c - v.y * s, v.x * s + v.y * c);
    ch.yaw_rate_rad_s *= 0.7;
}
