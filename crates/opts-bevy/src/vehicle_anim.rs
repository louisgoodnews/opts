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

/// The mesh's own origin (for `origin_from_mesh`) and whether its `.o3d` matrix swaps the y and z
/// axes (the matrix of the wheel meshes of the MAN New Lion's City family).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MeshPivot {
    /// Origin in OMSI vehicle axes.
    pub origin: Vec3,
    /// The matrix rotation is the y/z swap `[1,0,0|0,0,1|0,1,0]`.
    pub swaps_yz: bool,
}

/// The y/z swap as a matrix in OMSI axes.
fn swap_yz() -> Mat4 {
    Mat4::from_cols(
        Vec3::X.extend(0.0),
        Vec3::Z.extend(0.0),
        Vec3::Y.extend(0.0),
        Vec4::W,
    )
}

/// Whether a rotation animation needs the mesh matrix to get its vertical axis: with
/// `origin_from_mesh` on a y/z-swapping mesh, the origin rotation gives a horizontal axis that
/// the swap turns vertical (`origin_rot_z 90`: forward, then up). Axes that are vertical or
/// stay horizontal are left alone (`docs/src/architecture/animation-axes.md`).
fn needs_matrix(a: &ModelAnim, pivot: Option<MeshPivot>) -> bool {
    if !a.origin_from_mesh || a.origin_trans.is_some() || a.anim_rot.is_none() {
        return false;
    }
    if !pivot.is_some_and(|p| p.swaps_yz) {
        return false;
    }
    let axis = origin_rotation(a).transform_vector3(Vec3::X);
    axis.z.abs() < 0.5 && (swap_yz().transform_vector3(axis)).z.abs() > 0.97
}

fn origin_rotation(a: &ModelAnim) -> Mat4 {
    Mat4::from_rotation_x(a.origin_rot.x.to_radians())
        * Mat4::from_rotation_y(a.origin_rot.y.to_radians())
        * Mat4::from_rotation_z(a.origin_rot.z.to_radians())
}

fn frame_of(a: &ModelAnim, pivot: Option<MeshPivot>, with_matrix: bool) -> Mat4 {
    // An explicit `origin_trans` wins; `origin_from_mesh` takes the mesh's own origin (the
    // translation of its matrix chunk, e.g. a wheel's centre).
    let origin = a
        .origin_trans
        .or_else(|| a.origin_from_mesh.then_some(pivot.map(|p| p.origin)).flatten())
        .unwrap_or(Vec3::ZERO);
    let rot = origin_rotation(a);
    // The mesh matrix follows the origin rotation (the frame's axes are the rotated, then swapped).
    let matrix = if with_matrix { swap_yz() } else { Mat4::IDENTITY };
    Mat4::from_translation(origin) * matrix * rot
}

fn steps_of(anim: &ModelAnim, pivot: Option<MeshPivot>) -> Vec<AnimStep> {
    let frame = frame_of(anim, pivot, false);
    let rot_frame = if needs_matrix(anim, pivot) {
        frame_of(anim, pivot, true)
    } else {
        frame
    };
    let mk = |a: &AnimAction, translate: bool| AnimStep {
        frame: if translate { frame } else { rot_frame },
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

/// Logs once per mesh that its steering/rotation axis was composed with the mesh matrix.
fn warn_matrix_once(mesh: &str) {
    use std::collections::HashSet;
    use std::sync::{Mutex, OnceLock};
    static SEEN: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    let seen = SEEN.get_or_init(Mutex::default);
    if seen
        .lock()
        .is_ok_and(|mut s| s.insert(mesh.to_lowercase()))
    {
        tracing::warn!(
            mesh,
            "origin_from_mesh rotation axis composed with the mesh's y/z-swap matrix (animation-axes.md)"
        );
    }
}

impl MeshAnimation {
    /// Builds the rules of `mesh`; `None` when it has neither animation nor visibility rules.
    /// `pivot` is the mesh's own origin in OMSI vehicle axes (the matrix of its `.o3d`), used by
    /// animations declared `origin_from_mesh`.
    #[must_use]
    pub fn from_mesh(owner: Entity, mesh: &ModelMesh, pivot: Option<MeshPivot>) -> Option<Self> {
        if mesh.anims.iter().any(|a| needs_matrix(a, pivot)) {
            warn_matrix_once(&mesh.path);
        }
        let steps: Vec<AnimStep> = mesh.anims.iter().flat_map(|a| steps_of(a, pivot)).collect();
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

    /// Excerpts of the steering blocks of MAN_SD200 `model_SD77.cfg` (explicit `origin_trans`) and
    /// MAN_NewLionsCity `model_21C_4door_main.cfg` (`origin_from_mesh`, y/z-swap mesh matrix).
    const SD77: &str = "[mesh]\nwheel.o3d\n\n[newanim]\norigin_trans\n-0.95\n2.943\n0.495\norigin_rot_y\n90\nanim_rot\nAxle_Steering_0_L\n57.29578\n\n++++++\n";
    const NLC: &str = "[mesh]\nsolo\\rad_vl_felge.o3d\n\n[newanim]\norigin_from_mesh\norigin_rot_z\n90\nanim_rot\nAxle_Steering_0_L\n57.29578\n\n[newanim]\norigin_from_mesh\norigin_rot_y\n90\nanim_rot\nAxle_Steering_0_L\n4\n\n++++++\n";

    fn steering_mesh(cfg: &str, pivot: Option<MeshPivot>) -> MeshAnimation {
        let model = opts_core::formats::vehicles::model::ModelConfig::parse(cfg).expect("parses");
        MeshAnimation::from_mesh(Entity::PLACEHOLDER, &model.lods[0].meshes[0], pivot)
            .expect("animated")
    }

    /// Steering by 0.2 rad: a point 1 m ahead of the hub must move sideways and keep its height
    /// (a vertical steering axis), where a roll axis would leave it on the axis.
    fn steers_about_vertical(steps: &[AnimStep], hub: Vec3) -> bool {
        // Only the main steering step (the first).
        let m = animation_matrix(&steps[..1], |_| 0.2);
        let p = bevy(hub + Vec3::Y);
        let moved = m.transform_point3(p);
        (moved.x - p.x).abs() > 0.1 && (moved.y - p.y).abs() < 1e-4
    }

    #[test]
    fn explicit_origin_steering_is_vertical_sd77() {
        let hub = Vec3::new(-0.95, 2.943, 0.495);
        let anim = steering_mesh(SD77, None);
        assert!(steers_about_vertical(&anim.steps, hub));
    }

    #[test]
    fn swapped_matrix_mesh_steers_about_the_vertical_nlc() {
        let hub = Vec3::new(-1.0, 3.0, 0.5);
        let swapped = MeshPivot { origin: hub, swaps_yz: true };
        let anim = steering_mesh(NLC, Some(swapped));
        assert!(steers_about_vertical(&anim.steps, hub));
        // Without the swap (no matrix) the same data rolls about the forward axis (the bug).
        let plain = MeshPivot { origin: hub, swaps_yz: false };
        let anim = steering_mesh(NLC, Some(plain));
        assert!(!steers_about_vertical(&anim.steps, hub));
    }

    #[test]
    fn the_matrix_is_left_out_where_the_axis_is_already_vertical() {
        // The class `origin_rot_y 90` on a swapped mesh is not changed (the side-effect entry).
        let swapped = MeshPivot { origin: Vec3::ZERO, swaps_yz: true };
        let anim = steering_mesh(NLC, Some(swapped));
        // (factor 4: the variable is raised so that the angle is clearly measurable)
        let m = animation_matrix(&anim.steps[1..2], |_| 5.0);
        let p = bevy(Vec3::Y);
        let moved = m.transform_point3(p);
        assert!((moved.x - p.x).abs() > 0.1 && (moved.y - p.y).abs() < 1e-4);
    }
}
