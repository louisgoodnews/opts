//! Script-driven vehicle model animation (`[newanim]`, `[visible]`) — M-0045.
//!
//! Variables written by the vehicle scripts (or fed from the simulation) move and show meshes.
//! Conventions, with their evidence (`docs/src/formats/text/vehicles/model_cfg.md`):
//!
//! * OMSI turns in its own left-handed axes (x right, y up, z forward), which in the right-handed
//!   OMSI vehicle axes used here (x right, y forward, z up) means every angle with the
//!   **opposite sign** (`docs/src/architecture/animation-axes.md`, "One handedness");
//! * the animation frame is `origin_trans` plus the rotation `Rx·Ry·Rz` of `origin_rot_x/y/z`
//!   (degrees, each negated: `origin_rot_y 90` turns X **up**, the steering axis, `-90` down,
//!   the suspension's travel); the rotation order follows the tilted steering wheels (SD77
//!   `rot_x -10, rot_y 90`, Valiant `rot_x 30.7, rot_z 90`: the measured rim normals);
//! * `anim_rot` rotates about the frame's **X axis** and `anim_trans` moves along it (the model
//!   authors' comment: "no rotation needed, wheels turn about the X axis");
//! * the angle is `variable × factor` degrees, again with the opposite sign of a right-handed
//!   rotation: with `Wheel_Rotation` increasing while driving forward the wheel top moves forward,
//!   a positive `Axle_Steering` (a right turn) turns the wheels and the steering wheel right;
//! * several `[newanim]` blocks apply in declaration order, the first innermost (wheel: spin,
//!   then suspension, then steering);
//! * `maxspeed` limits how fast the animation follows its variable, in variable units per second
//!   (the wiki: `maxspeed` = 1 ÷ duration in seconds); the first update snaps to the value;
//! * `delay` damps the approach to the target: a first-order lag with rate `delay` per second
//!   (time constant `1 ÷ delay` s, the wiki's "1 ÷ x before the end"), evaluated exactly in `dt`
//!   (`shown += (target - shown) · (1 - e^(-delay·dt))`), so the curve does not depend on the frame
//!   rate. `maxspeed` then limits the resulting step. The units of `delay` are not verified against
//!   OMSI ❓ (see `model_cfg.md`).

use bevy::math::{Mat4, Vec3, Vec4};
use bevy::prelude::*;
use opts_core::formats::vehicles::model::{AnimAction, ModelAnim, ModelMesh};

use crate::ai_variables::{AiVariables, VariableSource};
use crate::vehicle_script::VehicleScripts;

/// One animation step: the frame in OMSI axes and what the variable does in it.
#[derive(Debug, Clone, PartialEq)]
pub struct AnimStep {
    /// Frame (origin and orientation) in OMSI vehicle axes.
    pub frame: Mat4,
    /// Driving variable.
    pub variable: String,
    /// Multiplier (degrees or metres per variable unit).
    pub factor: f32,
    /// `true`: translate along the frame's X axis; `false`: rotate about it.
    pub translate: bool,
    /// Largest change of the variable per second (`maxspeed`); `None` follows it at once.
    pub maxspeed: Option<f32>,
    /// Damping rate per second (`delay`, time constant `1 ÷ delay`); `None` follows undamped.
    pub delay: Option<f32>,
}

/// Animation and visibility rules of one mesh, driven by the vehicle's script variables.
#[derive(Component, Debug, Clone)]
pub struct MeshAnimation {
    /// The vehicle root that owns the script engine.
    pub owner: Entity,
    /// Steps in declaration order.
    pub steps: Vec<AnimStep>,
    /// `[visible]` rules: the mesh shows only while every variable equals its value.
    pub visible: Vec<(String, f32)>,
    /// Variable value each step currently shows (after `maxspeed`); empty before the first update.
    pub shown: Vec<f32>,
}

/// The mesh's own origin (for `origin_from_mesh`) and the rotation part of its `.o3d` matrix.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MeshPivot {
    /// Origin in OMSI vehicle axes.
    pub origin: Vec3,
    /// Rows of the matrix rotation (normalised, o3d coordinates): the images of the object's local
    /// axes in the parent.
    pub rows: [[f32; 3]; 3],
    /// The matrix's scale (the mean length of its rows): OMSI moves an `anim_trans` in the mesh's
    /// own frame, so a mesh exported with a scale of 0.01 (the MB C2 BVG's gear buttons, rows of
    /// about 0.0077) moves `0.8` by 7 mm, not 0.8 m.
    pub scale: f32,
}

mod pivot;
pub use pivot::*;

mod steps;
use steps::*;

mod api;
pub use api::*;

mod apply;
pub use apply::*;

/// Applies animation and visibility to every animated mesh.
pub fn vehicle_animation_system(
    time: Res<Time>,
    mut meshes: Query<(&mut MeshAnimation, &mut Transform, &mut Visibility)>,
    vehicles: Query<(
        Option<&VehicleScripts>,
        Option<&AiVariables>,
        Option<&crate::vehicle_sim::VehicleController>,
    )>,
) {
    let dt = time.delta_secs();
    for (mut anim, mut transform, mut visibility) in &mut meshes {
        let Ok((scripts, table, controller)) = vehicles.get(anim.owner) else {
            continue;
        };
        let Some(engine) = VariableSource::of(scripts, table) else {
            continue;
        };
        if !anim.steps.is_empty() {
            let body = controller.and_then(|c| c.sim.body.as_ref());
            let targets: Vec<f32> = anim
                .steps
                .iter()
                .map(|s| engine.get(&s.variable) - suspension_rest(body, &s.variable))
                .collect();
            anim.follow(&targets, dt);
            let m = animation_matrix_from_values(&anim.steps, &anim.shown);
            *transform = Transform::from_matrix(m);
        }
        if !anim.visible.is_empty() {
            let shown = anim
                .visible
                .iter()
                .all(|(name, expected)| (engine.get(name) - expected).abs() < 1e-3);
            let want = if shown {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
            if *visibility != want {
                *visibility = want;
            }
        }
    }
}

/// Registers [`vehicle_animation_system`] after the scripts ran.
#[derive(Default)]
pub struct VehicleAnimationPlugin;

impl Plugin for VehicleAnimationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            vehicle_animation_system.before(bevy::transform::TransformSystems::Propagate),
        );
    }
}

#[cfg(test)]
mod delay_tests;
#[cfg(test)]
mod tests;
