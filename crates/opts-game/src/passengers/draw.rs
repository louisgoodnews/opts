//! Drawing the passengers inside: walking along the cabin's paths, standing, or seated.

use bevy::prelude::*;
use opts_bevy::coords::omsi_to_bevy_pos;
use opts_bevy::{hip_offset, seat_hip_lift, Human, HumanPose};
use opts_sim::passengers::{Phase, WALK_SPEED_M_S};

use super::{facing, PassengerRig};
use crate::in_game::omsi_world::{advance_phase, apply_gait, apply_rest_pose, apply_seated_pose};

/// The root of a seated character facing `rotation`: its hip joint (`hip`, the hip pivot in the
/// character's own frame) `lift` above the seat's contact point `contact` (Bevy axes, the unit's
/// frame).
///
/// A `[passpos]` seat gives the point where the passenger's seat touches the vehicle ("Arsch" in
/// the files' own description), `h` above the floor; the hip joint lies above it by the
/// character's [`seat_hip_lift`] (its hip pivot minus `[seatheight]`). The character's root is
/// at its feet, so it goes down by the hip height; the legs are then fitted to `h`
/// ([`opts_bevy::seated_rotations`]) so that the feet rest on the floor.
#[must_use]
pub(super) fn seated_root(contact: Vec3, rotation: Quat, hip: Vec3, lift: f32) -> Vec3 {
    contact + Vec3::Y * lift - rotation * hip
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
            (Some(seat), Some(h)) => seated_root(
                omsi_to_bevy_pos(p.position),
                Quat::from_rotation_y(-seat.rotation_rad),
                hip_offset(&h.profile),
                seat_hip_lift(&h.profile),
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
        if let Some(seat) = settled_seat {
            t.rotation = Quat::from_rotation_y(-seat.rotation_rad);
        } else if let Some(q) = yaw.filter(|_| moving || d > 0.05) {
            t.rotation = q;
        }
        if let Some(h) = human {
            if let Some(seat) = settled_seat {
                apply_seated_pose(h, seat.height_m, transforms);
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
    fn a_seated_passenger_rests_on_the_seat_with_the_feet_on_the_floor() {
        // The trailer of the MAN NG 272 (GN92): floor at 0.632 m, a seat contact point at
        // 1.071 m (h = 0.439 m above the floor); a character with its hip joint 0.84 m above
        // its feet, its knee 0.43 m, its `[seatheight]` 0.75 m (the aXYZ figures).
        let profile = opts_core::formats::humans::HumanProfile::from_text(
            "[model]\nm.cfg\n[seatheight]\n0.75\n[humangeom]\n0.2\n1.75\n[links]\n0.10\n-0.01\n0.84\n0.14\n-0.02\n0.43\n-0.01\n1.05\n0.17\n-0.05\n1.39\n0.43\n-0.08\n1.34\n-0.05\n1.49\n0.69\n-0.08\n1.34\n0.89\n-0.08\n1.34\n[walk_param]\n1.4\n70\n0.7\n1.0\n0.0\n[voice]\nM2\n[mass]\n0.07\n",
        )
        .unwrap();
        let (floor, h) = (0.632_f32, 1.071_f32 - 0.632);
        let contact = omsi_to_bevy_pos(Vec3::new(-0.893, 2.556, 1.071));
        let hip = hip_offset(&profile);
        let lift = seat_hip_lift(&profile);
        assert!(
            (lift - 0.09).abs() < 1e-5,
            "the hip joint is above the cushion"
        );
        let pivots = opts_bevy::rest_pivots(&profile);
        let q = opts_bevy::seated_rotations(&profile, h);
        // The right sole in the root frame: hip, thigh, shin.
        let sole_local =
            pivots[4] + q[4] * ((pivots[6] - pivots[4]) + q[6] * Vec3::new(0.0, -pivots[6].y, 0.0));
        for rot in [0.0_f32, 90.0, 180.0] {
            let r = Quat::from_rotation_y(-rot.to_radians());
            let root = seated_root(contact, r, hip, lift);
            let hips = root + r * hip;
            assert!(
                (hips - (contact + Vec3::Y * lift)).length() < 1e-5,
                "the hips are on the seat"
            );
            let sole = root + r * sole_local;
            assert!(
                (sole.y - floor).abs() < 0.01,
                "rot {rot}: the feet rest on the floor ({} vs {floor})",
                sole.y
            );
        }
    }
}
