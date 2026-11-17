//! The trailer-front joint of an articulated bellow, driven every frame by the trailer's rigid
//! motion about its hitch (`opts_sim::vehicle::trailer_front::joint_transform`).
//!
//! The joint is an entity below the bellow's frame whose local transform is the joint matrix in
//! the Bevy model space of the meshes. With identity inverse bind poses its global transform is
//! the matrix the trailer-end vertices are posed with, so at rest it is the identity.

use bevy::math::DMat4;
use bevy::prelude::*;
use opts_sim::vehicle::trailer_front::joint_transform;
use opts_sim::VehicleSimulation;

use crate::vehicle_sim::VehicleController;

/// The trailer-front joint of a bellow of the vehicle `root`, hitched at `hitch` (OMSI, the head's
/// frame).
#[derive(Component, Debug, Clone, Copy)]
pub struct TrailerFrontJoint {
    /// The articulated vehicle whose simulation drives the joint.
    pub root: Entity,
    /// The hitch `H` in OMSI coordinates of the vehicle.
    pub hitch: bevy::math::DVec3,
}

/// The OMSI joint matrix `J` as a matrix in the Bevy model space: `B · J · Bᵀ`, with `B` the
/// vehicle's conversion `(x, y, z) -> (x, z, -y)` (`omsi_to_bevy_pos`).
#[must_use]
pub fn joint_model_transform(j: DMat4) -> Transform {
    let b = DMat4::from_cols(
        bevy::math::DVec4::X,
        bevy::math::DVec4::new(0.0, 0.0, -1.0, 0.0),
        bevy::math::DVec4::Y,
        bevy::math::DVec4::W,
    );
    Transform::from_matrix((b * j * b.transpose()).as_mat4())
}

/// The trailer-front transform of a simulation for the hitch `hitch`: the first trailer's yaw,
/// pitch and roll about the hitch. Identity when no trailer follows.
#[must_use]
pub fn trailer_front_transform(sim: &VehicleSimulation, hitch: bevy::math::DVec3) -> Transform {
    let (Some(alpha), Some((beta, gamma))) = (sim.consist_alpha_rad(1), sim.joint_angles_deg(1))
    else {
        return Transform::IDENTITY;
    };
    // The published yaw is counter-clockwise; `joint_transform` takes the clockwise one.
    let j = joint_transform(
        hitch,
        -alpha,
        f64::from(beta).to_radians(),
        f64::from(gamma).to_radians(),
    );
    joint_model_transform(j)
}

/// Sets every trailer-front joint from its vehicle's simulation, every frame.
pub fn trailer_front_system(
    roots: Query<&VehicleController>,
    mut joints: Query<(&TrailerFrontJoint, &mut Transform)>,
) {
    for (joint, mut tf) in &mut joints {
        let Ok(controller) = roots.get(joint.root) else {
            continue;
        };
        *tf = trailer_front_transform(&controller.sim, joint.hitch);
    }
}
