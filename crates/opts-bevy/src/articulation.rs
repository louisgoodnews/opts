//! Articulated buses and multi-car trams: following segments and the joint variables.
//!
//! The leading unit of a vehicle is the root entity driven by its [`crate::vehicle_sim::VehicleController`]; every
//! further segment is an entity of its own ([`ArticulatedSegment`]) listed in the root's
//! [`ArticulatedChain`]. The state of the following units lives in the root's
//! [`opts_sim::VehicleSimulation::consist`] (hitch on the leader's rear coupling, rear axle
//! rolling without side slip), which the simulation steps with the chassis; this module only
//! *reads* it each fixed step, hands the poses to the segment entities and writes the joint
//! angles into the script engine of the vehicle as OMSI's models expect them
//! (`docs/src/architecture/articulation.md`):
//!
//! * `articulation_<n>_alpha` — yaw of segment `n + 1` against the unit ahead, **degrees**,
//!   positive when the segment is turned counter-clockwise, a right-hand curve (the models rotate
//!   the bellows dummies by fractions `0.25 … 1.0` of it, with factors in degrees);
//! * `articulation_<n>_beta` / `_gamma` — pitch and roll of the unit against the one ahead,
//!   degrees, from the simulation's fixed-step ground following (`0` without ground samples).

mod chain;
mod scripts;
mod segment;
mod system;
mod trailer_joint;

pub use chain::{joint_beta_deg, ArticulatedChain};
pub use segment::{ArticulatedSegment, SegmentGround, SegmentWheel};
pub use system::{articulation_render_system, articulation_system};
pub use trailer_joint::{
    joint_model_transform, trailer_front_system, trailer_front_transform, TrailerFrontJoint,
};

use bevy::prelude::*;

/// A leader position change larger than `max(|v|·dt·2, JUMP_THRESHOLD_M)` in one step is a
/// position jump, not motion: larger than any per-step displacement the simulation produces at
/// any speed, below the 0.5 m steps of the spawn placement.
pub const JUMP_THRESHOLD_M: f32 = 0.3;

/// Registers [`articulation_system`] between the physics step and the scripts.
#[derive(Default)]
pub struct ArticulationPlugin;

impl Plugin for ArticulationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            articulation_system
                .after(crate::vehicle_sim::vehicle_physics_step_system)
                .after(crate::RailSet)
                .before(crate::vehicle_script::vehicle_script_system),
        )
        // After the ground follower of the game (Update) has set the root's height.
        .add_systems(
            PostUpdate,
            (
                articulation_render_system,
                trailer_front_system.after(articulation_render_system),
            )
                .before(TransformSystems::Propagate),
        );
    }
}
