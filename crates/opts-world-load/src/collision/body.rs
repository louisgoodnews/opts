//! The boxes of vehicles: the head unit's `.bus` box, the body of one part and the measured box
//! of an AI vehicle.

use bevy::prelude::*;

pub use opts_game_state::in_game::omsi_world::body::BodyBox;

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
