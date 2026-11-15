//! Seated poses: the hip lift on a seat, the thigh angle that puts the soles on the floor, and the seated rotations.

use super::*;

/// The most the hip joint is put above the point a seated character rests on (m); the corpus
/// lies between 0 and 0.14.
const MAX_SEAT_LIFT_M: f32 = 0.15;
/// The thigh angle of a fitted seated pose, from hanging straight down (rad): from sloping down
/// by 40° on a high seat (the feet then hang) to the knees 25° up on a low one.
pub(super) const THIGH_MIN_RAD: f32 = 50.0 * std::f32::consts::PI / 180.0;
pub(super) const THIGH_MAX_RAD: f32 = 115.0 * std::f32::consts::PI / 180.0;

/// How far the hip joint of a seated character lies above the point it rests on (m).
///
/// `[seatheight]` is the height, in the standing character's frame, of the part that rests on a
/// seat: in all 58 installed `.hum` files it lies 0–0.14 m below the hip joint (`[links]`), the
/// thickness of the thigh and buttocks under the hip joint. A `[passpos]` seat point is that
/// contact ("Arsch"), so the hip joint goes this much above it.
#[must_use]
pub fn seat_hip_lift(p: &HumanProfile) -> f32 {
    (p.links.hip.z - p.seat_height_m).clamp(0.0, MAX_SEAT_LIFT_M)
}

/// The thigh angle from hanging straight down (rad) at which the shin, hanging vertically from
/// the knee, puts the sole on the floor `hip_above_floor_m` below the hip joint; clamped to a
/// natural range (a high seat leaves the feet hanging, a low one raises the knees).
///
/// The thigh is the hip-to-knee vector in the sagittal plane of `[links]`, the shin and foot the
/// knee's height above the ground.
#[must_use]
pub fn seated_thigh_angle(p: &HumanProfile, hip_above_floor_m: f32) -> f32 {
    let dy = (p.links.hip.z - p.links.knee.z).max(0.05);
    let dz = p.links.hip.y - p.links.knee.y;
    let thigh = dy.hypot(dz);
    let shin = p.links.knee.z.max(0.05);
    (dz.atan2(dy) + ((hip_above_floor_m - shin) / thigh).clamp(-1.0, 1.0).acos())
        .clamp(THIGH_MIN_RAD, THIGH_MAX_RAD)
}

/// The seated pose fitted to a seat `seat_height_m` above the floor (the `h` of `[passpos]`):
/// thighs at [`seated_thigh_angle`], shins vertical, so that the feet rest on the floor. A seat
/// of unknown height (`0`) keeps the generic pose (thighs horizontal).
#[must_use]
pub fn seated_rotations(p: &HumanProfile, seat_height_m: f32) -> [Quat; 13] {
    let mut q = pose_rotations(p, HumanPose::Seated);
    if seat_height_m > 0.0 {
        let a = seated_thigh_angle(p, seat_height_m + seat_hip_lift(p));
        q[3] = Quat::from_rotation_x(a);
        q[4] = Quat::from_rotation_x(a);
        q[5] = Quat::from_rotation_x(-a);
        q[6] = Quat::from_rotation_x(-a);
    }
    q
}
