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
//!   then suspension, then steering);
//! * `maxspeed` limits how fast the animation follows its variable, in variable units per second
//!   (the wiki: `maxspeed` = 1 ÷ duration in seconds); the first update snaps to the value.
//!   `delay` (damping near the end) is not applied yet ❓.

use bevy::math::{Mat4, Vec3};
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

fn frame_of(a: &ModelAnim) -> Mat4 {
    let origin = a.origin_trans.unwrap_or(Vec3::ZERO);
    let rot = Mat4::from_rotation_x(a.origin_rot.x.to_radians())
        * Mat4::from_rotation_y(a.origin_rot.y.to_radians())
        * Mat4::from_rotation_z(a.origin_rot.z.to_radians());
    Mat4::from_translation(origin) * rot
}

fn steps_of(anim: &ModelAnim) -> Vec<AnimStep> {
    anim_steps(anim, None)
}

/// The steps of one `[newanim]` block; `origin` (OMSI axes) replaces `origin_trans` when given
/// (scenery `origin_from_mesh` uses the mesh file's pivot).
#[must_use]
pub fn anim_steps(anim: &ModelAnim, origin: Option<Vec3>) -> Vec<AnimStep> {
    let mut frame = frame_of(anim);
    if let Some(o) = origin {
        frame.w_axis = o.extend(1.0);
    }
    let mk = |a: &AnimAction, translate: bool| AnimStep {
        frame,
        variable: a.variable.clone(),
        factor: a.factor,
        translate,
        maxspeed: anim.maxspeed.filter(|v| *v > 0.0),
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
            shown: Vec::new(),
        })
    }

    /// Moves the shown values towards `targets` (one per step) over `dt` seconds, limited by each
    /// step's `maxspeed`; the first call adopts the targets.
    pub fn follow(&mut self, targets: &[f32], dt: f32) {
        if self.shown.len() != targets.len() {
            self.shown = targets.to_vec();
            return;
        }
        for ((shown, target), step) in self.shown.iter_mut().zip(targets).zip(&self.steps) {
            *shown = match step.maxspeed {
                Some(speed) => {
                    let max = speed * dt;
                    *shown + (target - *shown).clamp(-max, max)
                }
                None => *target,
            };
        }
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
    let values: Vec<f32> = steps.iter().map(|s| value(&s.variable)).collect();
    animation_matrix_from_values(steps, &values)
}

/// Like [`animation_matrix`] with the variable value of each step given directly.
#[must_use]
pub fn animation_matrix_from_values(steps: &[AnimStep], values: &[f32]) -> Mat4 {
    let mut m = Mat4::IDENTITY;
    for (s, value) in steps.iter().zip(values) {
        let v = value * s.factor;
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
    time: Res<Time>,
    mut meshes: Query<(&mut MeshAnimation, &mut Transform, &mut Visibility)>,
    vehicles: Query<(Option<&VehicleScripts>, Option<&AiVariables>)>,
) {
    let dt = time.delta_secs();
    for (mut anim, mut transform, mut visibility) in &mut meshes {
        let Ok((scripts, table)) = vehicles.get(anim.owner) else {
            continue;
        };
        let Some(engine) = VariableSource::of(scripts, table) else {
            continue;
        };
        if !anim.steps.is_empty() {
            let targets: Vec<f32> = anim.steps.iter().map(|s| engine.get(&s.variable)).collect();
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
mod tests {
    use super::*;

    fn spin(var: &str) -> AnimStep {
        AnimStep {
            frame: Mat4::from_translation(Vec3::new(1.2, 2.9, 0.5)),
            variable: var.into(),
            factor: 57.295_78,
            translate: false,
            maxspeed: None,
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
            maxspeed: None,
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
            maxspeed: None,
        };
        let m = animation_matrix(&[spin("w"), raise], |_| 1.0);
        let hub = bevy(Vec3::new(1.2, 2.9, 0.5));
        let p = m.transform_point3(hub);
        assert!((p.y - (hub.y + 1.0)).abs() < 1e-4, "{p:?}");
    }

    #[test]
    fn maxspeed_limits_how_fast_the_shown_value_follows() {
        let mut step = spin("door");
        step.maxspeed = Some(0.5);
        let mut anim = MeshAnimation {
            owner: Entity::PLACEHOLDER,
            steps: vec![step, spin("other")],
            visible: Vec::new(),
            shown: Vec::new(),
        };
        anim.follow(&[0.0, 0.0], 0.1);
        assert_eq!(anim.shown, [0.0, 0.0]);
        // The door opens in two seconds; the unlimited step jumps.
        anim.follow(&[1.0, 1.0], 1.0);
        assert!((anim.shown[0] - 0.5).abs() < 1e-6 && anim.shown[1] == 1.0);
        anim.follow(&[1.0, 1.0], 1.0);
        anim.follow(&[1.0, 1.0], 1.0);
        assert!(
            (anim.shown[0] - 1.0).abs() < 1e-6,
            "no overshoot: {:?}",
            anim.shown
        );
    }

    #[test]
    fn zero_variables_leave_the_mesh_in_place() {
        let m = animation_matrix(&[spin("w")], |_| 0.0);
        assert!(m.abs_diff_eq(Mat4::IDENTITY, 1e-6));
    }
}
