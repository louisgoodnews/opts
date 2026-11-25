//! Helpers of the collision pass: the following parts' poses, the log of a push and the scrape of
//! the velocity along the obstacles.

use bevy::prelude::*;
use opts_bevy::ArticulatedSegment;
use opts_collision::{respond, Obb};

use super::super::BodyBox;
use super::{FRICTION, RESTITUTION};

/// The following parts' poses with their boxes: (box, position, heading). Read from the
/// simulation's consist, so a push made earlier in this pass is already part of the pose.
pub(super) fn follower_poses(
    sim: &opts_sim::VehicleSimulation,
    chain: Option<&opts_bevy::ArticulatedChain>,
    part_boxes: &[BodyBox],
    segments: &Query<&ArticulatedSegment>,
) -> Vec<(BodyBox, Vec2, f32)> {
    let Some(c) = chain else {
        return Vec::new();
    };
    c.segments
        .iter()
        .zip(part_boxes)
        .enumerate()
        .filter_map(|(k, (e, b))| {
            let seg = segments.get(*e).ok()?;
            seg.kinematic_pose()?;
            let unit = sim.consist.get(k)?;
            let turn = if seg.reversed { std::f64::consts::PI } else { 0.0 };
            let h = (unit.heading + turn) as f32;
            Some((*b, unit.origin.as_vec2(), h))
        })
        .collect()
}

/// The contact decision of one push: the pair of bodies, the contact normal, the penetration and
/// the push taken, and the response chosen (a chain push for a following part, a head push for
/// the head itself).
pub(super) fn log_contact(
    part: usize,
    mover: Obb,
    other: Option<Obb>,
    normal: Vec2,
    depth: f32,
    push: f32,
    traffic: bool,
) {
    tracing::debug!(
        part,
        mover = ?mover.centre,
        other = ?other.map(|o| o.centre),
        normal = ?normal,
        depth,
        push,
        traffic,
        response = if part == 0 { "head push" } else { "chain push" },
        "collision contact"
    );
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
