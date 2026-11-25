//! The box of a vehicle part: its footprint in its own frame, from the `.bus` `[boundingbox]`.

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
