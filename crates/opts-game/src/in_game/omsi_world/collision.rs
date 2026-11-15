//! Collisions of the player's vehicle: with the scenery objects' collision meshes, with AI
//! vehicles, and with steps in the ground (walls, embankments, platform edges).
//!
//! * **Objects.** Every placed `.sco` with `[collision_mesh]` registers the bounding box of that
//!   mesh as an oriented box (`opts_sim::collision`) in [`CollisionIndex`] when its tile loads and
//!   leaves with the tile. A box whose top is no higher than a step above the vehicle's wheels
//!   (kerb, road plate) is driven over, one above the vehicle is passed under.
//! * **AI vehicles.** The box of an AI vehicle is measured from its meshes once and follows it;
//!   it is solid for the player, which is pushed out (the AI agent itself is kinematic).
//! * **Ground steps.** Points around the footprint are compared with the ground a little closer
//!   to the centre; a rise of more than a kerb over a short distance blocks the vehicle.
//! * **Rail vehicles** are ghosts for others, as in OMSI (Tram SDK): trams neither collide nor
//!   are pushed.
//!
//! The response moves the chassis out of the overlap (at most half a metre per step), removes the
//! velocity into the obstacle and scrapes a little off the rest.

mod body;
mod bounds;
mod boxes;
mod index;
mod mesh_boxes;
mod obstacles;
mod player;
mod register;

pub use body::{BodyBox, PlayerBox, VehicleCollider};
pub(super) use bounds::BoundsCache;
pub use index::CollisionIndex;
pub use obstacles::{attach_traffic_colliders, publish_traffic_obstacles};
pub use player::player_collisions;
pub(super) use register::{register_object, unregister_tile};

use boxes::{obb_of, player_boxes};
use obstacles::{GroundPlayer, PlayerCollide};

#[cfg(test)]
use bevy::prelude::*;
#[cfg(test)]
use obstacles::MAX_VEHICLE_HALF;

#[cfg(test)]
mod tests;
