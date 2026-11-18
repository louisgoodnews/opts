//! The placement of a segment's model under its ground pose (pitch, roll and yaw about its coupling).

use bevy::math::Vec3;
use bevy::prelude::Quat;

use crate::coords::omsi_to_bevy_pos;

/// The Bevy translation and rotation of a segment's model: its origin at `origin` (OMSI, ground
/// height included), turned by `yaw` about the vertical (Bevy `Y`, the render's `-h`, with any
/// half turn of a reversed segment already added), pitched by `pitch` and rolled by `roll` (rad).
///
/// The tilt pivots about the segment's own front coupling `front` (OMSI, model frame): the coupling
/// stays where the level placement puts it for every pitch and roll, so it lands on the hitch the
/// trailer-front joint pins (the joint fixes the hitch for every angle). At zero tilt this is the
/// placement at `origin`.
#[must_use]
pub fn segment_placement(
    origin: Vec3,
    front: Vec3,
    yaw: f32,
    pitch: f32,
    roll: f32,
) -> (Vec3, Quat) {
    let level = Quat::from_rotation_y(yaw);
    let rotation = level * Quat::from_rotation_x(pitch) * Quat::from_rotation_z(roll);
    let local = omsi_to_bevy_pos(front);
    let translation = omsi_to_bevy_pos(origin) + level * local - rotation * local;
    (translation, rotation)
}

#[cfg(test)]
mod tests;
