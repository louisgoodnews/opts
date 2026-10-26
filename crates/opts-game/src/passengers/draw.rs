//! Drawing the passengers inside: walking along the cabin's paths, standing, or seated.

use bevy::prelude::*;
use opts_bevy::coords::omsi_to_bevy_pos;
use opts_bevy::{hip_offset, Human, HumanPose};
use opts_sim::passengers::{Phase, WALK_SPEED_M_S};

use super::{facing, PassengerRig};
use crate::in_game::omsi_world::{advance_phase, apply_gait, apply_rest_pose};

/// The root of a seated character facing `rotation`: its hip joint (`hip`, the hip pivot in the
/// character's own frame) on the seat's contact point `contact` (Bevy axes, the unit's frame).
///
/// A `[passpos]` seat gives the point where the passenger's seat touches the vehicle ("Arsch" in
/// the files' own description), `h` above the floor; the character's root is at its feet, so it
/// goes down by the hip height, as the driver at `[drivpos]` does.
#[must_use]
pub(super) fn seated_root(contact: Vec3, rotation: Quat, hip: Vec3) -> Vec3 {
    contact - rotation * hip
}

/// Moves and poses the figures of the passengers inside after the simulation's positions.
pub(super) fn draw_inside(
    rig: &mut PassengerRig,
    commands: &mut Commands,
    humans: &Query<&Human>,
    transforms: &mut Query<&mut Transform>,
    dt: f32,
) {
    for p in rig.sim.passengers() {
        let Some(s) = rig.actors.get_mut(&p.id) else {
            continue;
        };
        let human = humans.get(s.entity).ok();
        let settled_seat = matches!(p.phase, Phase::Settled { .. })
            .then_some(s.seated)
            .flatten();
        let target = match (settled_seat, human) {
            (Some(rot), Some(h)) => seated_root(
                omsi_to_bevy_pos(p.position),
                Quat::from_rotation_y(-rot),
                hip_offset(&h.profile),
            ),
            _ => omsi_to_bevy_pos(p.position),
        };
        if p.unit != s.unit {
            // Through the joint: the person now belongs to the other unit's frame.
            if let Some(u) = rig.units.get(p.unit) {
                commands.entity(s.entity).insert(ChildOf(u.root));
                s.local = target;
                s.unit = p.unit;
            }
        }
        let moving = matches!(
            p.phase,
            Phase::Walking { .. } | Phase::WaitingOutside { .. }
        );
        let delta = target - s.local;
        let d = delta.length();
        let step = WALK_SPEED_M_S * 1.5 * dt;
        let yaw = if d > 0.02 { Some(facing(delta)) } else { None };
        s.local = if d <= step {
            target
        } else {
            s.local + delta / d * step
        };
        let Ok(mut t) = transforms.get_mut(s.entity) else {
            continue;
        };
        t.translation = s.local;
        // Sitting down is a step onto the seat, not a walk: the figure keeps facing the seat's way.
        if let Some(rot) = settled_seat {
            t.rotation = Quat::from_rotation_y(-rot);
        } else if let Some(q) = yaw.filter(|_| moving || d > 0.05) {
            t.rotation = q;
        }
        if let Some(h) = human {
            if settled_seat.is_some() {
                apply_rest_pose(h, HumanPose::Seated, transforms);
            } else if d > 0.02 {
                s.phase = advance_phase(h, s.phase, WALK_SPEED_M_S, dt);
                apply_gait(h, s.leg_m, s.phase, transforms);
            } else {
                apply_rest_pose(h, HumanPose::Standing, transforms);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_seated_passenger_has_the_hips_on_the_seat_and_the_feet_below_it() {
        // The trailer of the MAN NG 272 (GN92): floor at 0.632 m, a seat contact point at
        // 1.071 m (0.434 m above the floor); a character with its hip joint 0.95 m above its feet.
        let contact = omsi_to_bevy_pos(Vec3::new(-0.893, 2.556, 1.071));
        let hip = Vec3::new(0.0, 0.95, 0.02);
        for rot in [0.0_f32, 90.0, 180.0] {
            let q = Quat::from_rotation_y(-rot.to_radians());
            let root = seated_root(contact, q, hip);
            let hips = root + q * hip;
            assert!(hips.distance(contact) < 1e-5, "the hips are on the seat");
            assert!(
                (root.y - (1.071 - 0.95)).abs() < 1e-5,
                "the root goes down by the hip height"
            );
            assert!(root.y < 0.632, "the feet are not above the floor");
        }
    }
}
