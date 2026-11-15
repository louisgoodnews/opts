//! The boxes of vehicles: the head unit's `.bus` box, the body of one part and the measured box
//! of an AI vehicle.

use bevy::prelude::*;

/// The footprint of a vehicle part in its own frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BodyBox {
    /// Half extents `(right, forward)` (m).
    pub half: Vec2,
    /// Centre offset `(right, forward)` (m).
    pub offset: Vec2,
    /// Height of the body above the ground (m).
    pub height: f32,
}

impl BodyBox {
    /// From a `.bus` `[boundingbox]` (full extents and centre offset, x lateral, y longitudinal).
    #[must_use]
    pub fn from_bounding_box(size: Vec3, centre: Vec3) -> Self {
        Self {
            half: Vec2::new(size.x, size.y) * 0.5,
            offset: Vec2::new(centre.x, centre.y),
            height: size.z.max(1.0),
        }
    }
}

/// The box of the player's head unit (`[boundingbox]` of its `.bus`).
#[derive(Component, Debug, Clone, Copy)]
pub struct PlayerBox(pub BodyBox);

/// The measured box of an AI vehicle, in its own frame: centre and half extents of its meshes
/// `(right, up, back)` as Bevy sees them.
#[derive(Component, Debug, Clone, Copy)]
pub struct VehicleCollider {
    pub(super) centre: Vec3,
    pub(super) half: Vec3,
}
