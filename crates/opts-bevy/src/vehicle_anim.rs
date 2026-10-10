//! Script-driven vehicle model animation (`[newanim]`, `[visible]`) — M-0045.
//!
//! Variables written by the vehicle scripts (or fed from the simulation) move and show meshes.
//! Conventions, with their evidence (`docs/src/formats/text/vehicles/model_cfg.md`):
//!
//! * the animation frame is `origin_trans` plus the rotation `Rx·Ry·Rz` of `origin_rot_x/y/z`
//!   (degrees, right-handed, in OMSI axes: x right, y forward, z up); the rotation order is
//!   inferred from the SD77 steering wheel (`rot_x = -10`, `rot_y = 90`: a nearly vertical
//!   column leaning 10°) ❓;
//! * `anim_rot` rotates about the frame's **X axis** and `anim_trans` moves along it (the model
//!   authors' comment: "no rotation needed, wheels turn about the X axis"; suspension uses
//!   `origin_rot_y = -90` to make X point up, steering `+90` for the vertical steering axis);
//! * the angle is `variable × factor` degrees applied with the **opposite sign** of a
//!   right-handed rotation: with `Wheel_Rotation` increasing while driving forward the wheel top
//!   must move forward. The sign for steering is unverified ❓;
//! * several `[newanim]` blocks apply in declaration order, the first innermost (wheel: spin,
//!   then suspension, then steering).

use bevy::math::{Mat4, Vec3};
use bevy::prelude::*;
use opts_core::formats::vehicles::model::{AnimAction, ModelAnim, ModelMesh};

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
}

fn frame_of(a: &ModelAnim) -> Mat4 {
    let origin = a.origin_trans.unwrap_or(Vec3::ZERO);
    let rot = Mat4::from_rotation_x(a.origin_rot.x.to_radians())
        * Mat4::from_rotation_y(a.origin_rot.y.to_radians())
        * Mat4::from_rotation_z(a.origin_rot.z.to_radians());
    Mat4::from_translation(origin) * rot
}

fn steps_of(anim: &ModelAnim) -> Vec<AnimStep> {
    let frame = frame_of(anim);
    let mk = |a: &AnimAction, translate: bool| AnimStep {
        frame,
        variable: a.variable.clone(),
        factor: a.factor,
        translate,
    };
    // Translation first would move the rotation axis; rotation is the inner step.
    anim.anim_rot
        .iter()
        .map(|a| mk(a, false))
        .chain(anim.anim_trans.iter().map(|a| mk(a, true)))
        .collect()
}

impl MeshAnimation {
    /// Builds the rules of `mesh`; `None` when it has neither animation nor visibility rules.
    #[must_use]
    pub fn from_mesh(owner: Entity, mesh: &ModelMesh) -> Option<Self> {
        let steps: Vec<AnimStep> = mesh.anims.iter().flat_map(steps_of).collect();
        let visible: Vec<(String, f32)> = mesh
            .visibility
            .iter()
            .map(|v| (v.variable.clone(), v.expected_value))
            .collect();
        (!steps.is_empty() || !visible.is_empty()).then_some(Self {
            owner,
            steps,
            visible,
        })
    }
}

/// OMSI axes (x right, y forward, z up) to Bevy axes (x right, y up, z backward).
fn omsi_to_bevy_basis() -> Mat4 {
    Mat4::from_cols(
        Vec3::X.extend(0.0),
        (-Vec3::Z).extend(0.0),
        Vec3::Y.extend(0.0),
        Vec4::W,
    )
}

/// The mesh transform for the current variable values, in Bevy axes (the mesh vertices are
/// already converted to Bevy axes, so the OMSI-space motion is conjugated with the basis change).
#[must_use]
pub fn animation_matrix(steps: &[AnimStep], value: impl Fn(&str) -> f32) -> Mat4 {
    let mut m = Mat4::IDENTITY;
    for s in steps {
        let v = value(&s.variable) * s.factor;
        let a = if s.translate {
            Mat4::from_translation(Vec3::X * v)
        } else {
            Mat4::from_rotation_x(-v.to_radians())
        };
        m = s.frame * a * s.frame.inverse() * m;
    }
    let c = omsi_to_bevy_basis();
    c * m * c.inverse()
}

/// Applies animation and visibility to every animated mesh.
pub fn vehicle_animation_system(
    mut meshes: Query<(&MeshAnimation, &mut Transform, &mut Visibility)>,
    vehicles: Query<&VehicleScripts>,
) {
    for (anim, mut transform, mut visibility) in &mut meshes {
        let Ok(scripts) = vehicles.get(anim.owner) else {
            continue;
        };
        let Ok(engine) = scripts.engine.lock() else {
            continue;
        };
        if !anim.steps.is_empty() {
            let m = animation_matrix(&anim.steps, |name| engine.get(name));
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
mod tests {
    use super::*;

    fn spin(var: &str) -> AnimStep {
        AnimStep {
            frame: Mat4::from_translation(Vec3::new(1.2, 2.9, 0.5)),
            variable: var.into(),
            factor: 57.295_78,
            translate: false,
        }
    }

    /// A point of the mesh in Bevy axes from OMSI coordinates.
    fn bevy(p: Vec3) -> Vec3 {
        Vec3::new(p.x, p.z, -p.y)
    }

    #[test]
    fn wheel_top_moves_forward_when_the_rotation_variable_grows() {
        let top = bevy(Vec3::new(1.2, 2.9, 1.0));
        let m = animation_matrix(&[spin("w")], |_| 0.5);
        let moved = m.transform_point3(top);
        // Forward in OMSI is -z in Bevy.
        assert!(moved.z < top.z - 0.1, "top moved to {moved:?}");
        assert!((moved.y - top.y).abs() < 0.5);
        // The hub (on the axis) stays.
        let hub = bevy(Vec3::new(1.2, 2.9, 0.5));
        assert!(m.transform_point3(hub).distance(hub) < 1e-5);
    }

    #[test]
    fn suspension_with_rot_y_minus_90_moves_the_wheel_up() {
        let step = AnimStep {
            frame: Mat4::from_rotation_y((-90.0f32).to_radians()),
            variable: "s".into(),
            factor: 1.0,
            translate: true,
        };
        let m = animation_matrix(&[step], |_| 0.1);
        let p = m.transform_point3(Vec3::ZERO);
        assert!(
            (p.y - 0.1).abs() < 1e-5 && p.x.abs() < 1e-5 && p.z.abs() < 1e-5,
            "{p:?}"
        );
    }

    #[test]
    fn first_step_is_innermost() {
        // Spin then raise by one metre: the raise is not rotated by the spin.
        let raise = AnimStep {
            frame: Mat4::from_rotation_y((-90.0f32).to_radians()),
            variable: "s".into(),
            factor: 1.0,
            translate: true,
        };
        let m = animation_matrix(&[spin("w"), raise], |n| if n == "w" { 1.0 } else { 1.0 });
        let hub = bevy(Vec3::new(1.2, 2.9, 0.5));
        let p = m.transform_point3(hub);
        assert!((p.y - (hub.y + 1.0)).abs() < 1e-4, "{p:?}");
    }

    #[test]
    fn zero_variables_leave_the_mesh_in_place() {
        let m = animation_matrix(&[spin("w")], |_| 0.0);
        assert!(m.abs_diff_eq(Mat4::IDENTITY, 1e-6));
    }
}
