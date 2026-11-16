//! Door slots: one open door (its picture) and the grouping of doors that share one opening.

use super::*;


/// One picture: a door with its camera and image.
#[derive(Debug, Clone)]
pub struct DoorSlot {
    /// Index into the vehicle's [`DoorPoints`].
    pub door: usize,
    /// The vehicle and the camera that renders for it.
    pub vehicle: Entity,
    /// The camera entity.
    pub camera: Entity,
    /// The picture.
    pub image: Handle<Image>,
    /// Real time the door was seen closed (the fade starts [`FADE_DELAY_S`] later).
    pub closed_since: Option<f64>,
    /// Opacity of the picture, `1` to `0`.
    pub fade: f32,
}

/// Door points closer than this along the vehicle (m), on the same side, are one door (the
/// two lanes of an entrance, or the two leaves).
pub const SAME_DOOR_M: f32 = 1.6;

/// The first point of the door that `door` belongs to: the points of one entrance or exit lie
/// within [`SAME_DOOR_M`] of each other on the same side, and one opening shows one picture.
#[must_use]
pub fn door_group(doors: &[DoorPoint], door: usize) -> usize {
    let Some(d) = doors.get(door) else {
        return door;
    };
    doors
        .iter()
        .position(|o| {
            o.unit == d.unit
                && o.side == d.side
                && (o.position.z - d.position.z).abs() < SAME_DOOR_M
        })
        .unwrap_or(door)
}
