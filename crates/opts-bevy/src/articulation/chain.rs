//! The chain of a root unit and the consist seeding and position-jump detection of its simulation.

use bevy::math::DVec2;
use bevy::prelude::*;

use super::segment::ArticulatedSegment;
use super::JUMP_THRESHOLD_M;

/// The bellows variable `articulation_<k>_beta` (degrees) for a unit pitched `pitch` (rad, nose up
/// positive) behind a unit pitched `leader_pitch`. The bellows dummies turn about the lateral axis
/// by `-beta·factor` (right-handed, the `anim_rot` convention of wheels and `alpha`), so the end of
/// the bellows joined to the follower (behind the joint) only follows it when a nose-up follower
/// gives a **negative** beta (`docs/src/architecture/bellows-census.md`, Sign Verification).
#[must_use]
pub fn joint_beta_deg(leader_pitch: f32, pitch: f32) -> f32 {
    opts_sim::vehicle::ground::joint_beta_deg(leader_pitch, pitch)
}

/// The segments behind the root unit, front to back.
#[derive(Component, Debug, Clone, Default)]
pub struct ArticulatedChain {
    /// Segment entities in order.
    pub segments: Vec<Entity>,
}

/// Builds the consist of a root whose simulation has none (or a different number of units than
/// its chain) from the geometry the segments carry. A restored simulation that already holds its
/// consist is left alone.
pub(super) fn seed_consist(
    sim: &mut opts_sim::VehicleSimulation,
    chain: &[Entity],
    segments: &Query<(&mut ArticulatedSegment, &mut Transform), Without<ArticulatedChain>>,
) {
    sim.clear_consist();
    for &e in chain {
        let Ok((seg, _)) = segments.get(e) else {
            break;
        };
        let (hitch, front) = (seg.hitch_local, seg.front_local);
        sim.attach_trailer(
            DVec2::new(f64::from(hitch.x), f64::from(hitch.y)),
            DVec2::new(f64::from(front.x), f64::from(front.y)),
            f64::from((front.y - seg.rear_axle_y).max(1.0)),
            f64::from(seg.max_alpha_deg),
            seg.load.clone(),
        );
    }
}

pub(super) fn detect_position_jump(
    sim: &mut opts_sim::VehicleSimulation,
    root: Entity,
    last_leader: &mut std::collections::HashMap<Entity, (Vec2, u32)>,
    dt: f64,
) -> bool {
    let chassis = &sim.chassis;
    let seen = last_leader.insert(root, (chassis.position, sim.teleports));
    let announced = seen.is_none_or(|(_, n)| n != sim.teleports);
    let unannounced = seen.is_some_and(|(prev, _)| {
        let moved = (chassis.position - prev).length();
        moved > (chassis.forward_speed_m_s().abs() * dt as f32 * 2.0).max(JUMP_THRESHOLD_M)
    });
    if unannounced && !announced {
        tracing::debug!(
            ?root,
            "unannounced position jump of a chain leader; consist re-seeded"
        );
        sim.reseed_consist();
    }
    announced || unannounced
}
