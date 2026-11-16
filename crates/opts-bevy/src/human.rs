//! Human characters: skeleton from `[links]`, skinned meshes, poses and the head anchor.
//!
//! A `.hum` profile gives joint pivots; the skinned `.o3d` (chunk `0x54`) binds its vertices to
//! 13 canonical bones. The rest pose of the meshes is a T-pose (hands 0.7–0.9 m to the side), so
//! bind poses are pure translations to the pivots and a pose is a set of joint rotations.

use bevy::camera::visibility::NoFrustumCulling;
use bevy::ecs::component::Component;
use bevy::ecs::entity::Entity;
use bevy::ecs::name::Name;
use bevy::ecs::system::Commands;
use bevy::math::{Mat4, Quat, Vec3};
use bevy::mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes};
use bevy::mesh::Mesh3d;
use bevy::pbr::MeshMaterial3d;
use bevy::prelude::{Assets, Transform};

use opts_core::formats::humans::{HumanProfile, EYE_BELOW_CROWN_M};

use crate::coords::omsi_to_bevy_pos;
use crate::model::ModelPart;

mod eyes;
mod pose;
mod seat;
mod spawn;

pub use eyes::*;
pub use pose::*;
pub use seat::*;
pub use spawn::*;

/// The 13 canonical bones in joint-index order.
pub const HUMAN_BONES: [&str; 13] = [
    "Hip", "Main", "Head", "OS_L", "OS_R", "US_L", "US_R", "OA_L", "OA_R", "UA_L", "UA_R",
    "Hand_L", "Hand_R",
];

/// Parent bone of each bone (`None` for the root).
const PARENT: [Option<usize>; 13] = [
    None,    // Hip
    Some(0), // Main
    Some(1), // Head
    Some(0), // OS_L
    Some(0), // OS_R
    Some(3), // US_L
    Some(4), // US_R
    Some(1), // OA_L
    Some(1), // OA_R
    Some(7), // UA_L
    Some(8), // UA_R
    Some(9), // UA_L -> Hand_L
    Some(10),
];

/// Pose of a character.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HumanPose {
    /// Upright, arms hanging at the resting flare angle.
    Standing,
    /// Hips on a seat, thighs horizontal, shins vertical.
    Seated,
}

/// Joint entities of a spawned human.
#[derive(Debug, Clone)]
pub struct HumanRig {
    /// Joint entities in [`HUMAN_BONES`] order.
    pub joints: [Entity; 13],
    /// Eye anchor, a child of the head joint; cameras attach here.
    pub head_anchor: Entity,
}

/// Component on the root entity of a human.
#[derive(Component, Debug, Clone)]
pub struct Human {
    /// The `.hum` profile the character was built from.
    pub profile: HumanProfile,
    /// Joints and head anchor.
    pub rig: HumanRig,
}

/// Marker for the eye anchor entity of every human (the player's and the pedestrians').
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct HumanHead;

/// Marker for the eye anchor of the **player's** human: the ego camera attaches here.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct PlayerHead;

/// Marker for the eye anchor of the human that sits in **a** session vehicle's driver seat (the
/// active one or a parked one). [`sync_player_head`] keeps [`PlayerHead`] on the one whose vehicle
/// is the [`crate::vehicle_sim::PlayerVehicle`], so the ego and cockpit cameras always find exactly
/// one head.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct DriverHead;

/// On the root of the player's human (a child of the vehicle): once its pose is in place, the
/// character is shifted so that its eyes lie at `target` (vehicle-local Bevy space), where the
/// vehicle's driver camera is. The camera and the head then coincide, in the cockpit view and in
/// the ego view alike.
#[derive(Component, Debug, Clone, Copy)]
pub struct AlignEyes {
    /// The eye position in the vehicle's frame.
    pub target: Vec3,
    /// Frames to wait for the first transform propagation of the new hierarchy.
    pub wait: u8,
}

#[cfg(test)]
mod tests;
