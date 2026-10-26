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

/// Rows of the y/z swap `[1,0,0|0,0,1|0,1,0]` (the wheel meshes of the MAN New Lion's City).
pub const SWAP_YZ_ROWS: [[f32; 3]; 3] = [[1.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, 1.0, 0.0]];

impl MeshPivot {
    /// A pivot with an identity matrix rotation.
    #[must_use]
    pub fn plain(origin: Vec3) -> Self {
        Self {
            origin,
            rows: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            scale: 1.0,
        }
    }

    fn is_identity(&self) -> bool {
        (0..3).all(|i| (0..3).all(|j| (self.rows[i][j] - f32::from(u8::from(i == j))).abs() < 0.02))
    }

    fn is_swap_yz(&self) -> bool {
        (0..3).all(|i| (0..3).all(|j| (self.rows[i][j] - SWAP_YZ_ROWS[i][j]).abs() < 0.02))
    }

    /// The matrix rotation in vehicle axes, `P = C·Mᵀ·C` with `C` the y/z swap between o3d and
    /// vehicle axes: an axis `a` of the origin rotation becomes the object's axis `P·a`.
    fn vehicle_rotation(&self) -> Mat4 {
        let r = self.rows;
        // M has the rows r; Mᵀ has them as columns; conjugation with C swaps indices 1 and 2.
        let c = [0usize, 2, 1];
        // P[i][j] = Mᵀ[c(i)][c(j)] = M[c(j)][c(i)] = r[c(j)][c(i)]; column j of P:
        let column = |j: usize| Vec3::new(r[c[j]][c[0]], r[c[j]][c[1]], r[c[j]][c[2]]);
        Mat4::from_cols(
            column(0).extend(0.0),
            column(1).extend(0.0),
            column(2).extend(0.0),
            Vec4::W,
        )
    }
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

/// The mesh matrix rotation to compose after the origin rotation of a rotation animation, if it
/// is needed (`docs/src/architecture/animation-axes.md`). Only `origin_from_mesh` animations
/// without `origin_trans` with an `anim_rot` qualify, and
///
/// * on a y/z-swapping matrix (NLC wheels): only when the origin rotation gives a horizontal axis
///   that the swap turns vertical (`origin_rot_z 90`: forward, then up);
/// * on any other non-identity matrix (steering wheels): when the matrix changes the axis, the
///   origin rotation has no x part (the measured counter examples all rotate about x), and the
///   result is the mesh's own axis (checked against the vertices of 575 steering wheels).
fn matrix_rotation(a: &ModelAnim, pivot: Option<MeshPivot>) -> Option<Mat4> {
    let pivot = pivot?;
    if !a.origin_from_mesh
        || a.origin_trans.is_some()
        || a.anim_rot.is_none()
        || pivot.is_identity()
    {
        return None;
    }
    let axis = origin_rotation(a).transform_vector3(Vec3::X);
    let p = pivot.vehicle_rotation();
    if pivot.is_swap_yz() {
        let swapped = swap_yz().transform_vector3(axis);
        return (axis.z.abs() < 0.5 && swapped.z.abs() > 0.97).then(swap_yz);
    }
    let moved = p.transform_vector3(axis);
    (a.origin_rot.x == 0.0 && axis.dot(moved).abs() < 0.95).then_some(p)
}

/// Whether a translation of a y/z-swapping mesh (the NLC) is taken along the swapped axis: the
/// sliding leaves of its outward-swinging sliding doors say `origin_rot_y 90` (the X axis down)
/// and `anim_trans door_0_schiebe 0.47`, which with the swap runs along the bus, not down.
fn swaps_translation(a: &ModelAnim, pivot: Option<MeshPivot>) -> bool {
    pivot.is_some_and(|p| p.is_swap_yz())
        && a.origin_from_mesh
        && a.origin_trans.is_none()
        && a.anim_trans.is_some()
        && origin_rotation(a).transform_vector3(Vec3::X).z.abs() > 0.97
}

fn needs_matrix(a: &ModelAnim, pivot: Option<MeshPivot>) -> bool {
    matrix_rotation(a, pivot).is_some()
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
        .or_else(|| {
            a.origin_from_mesh
                .then_some(pivot.map(|p| p.origin))
                .flatten()
        })
        .unwrap_or(Vec3::ZERO);
    let rot = origin_rotation(a);
    // The mesh matrix follows the origin rotation (the frame's axes are the rotated, then swapped).
    let matrix = if with_matrix {
        matrix_rotation(a, pivot).unwrap_or(Mat4::IDENTITY)
    } else {
        Mat4::IDENTITY
    };
    Mat4::from_translation(origin) * matrix * rot
}

fn steps_of(anim: &ModelAnim, pivot: Option<MeshPivot>) -> Vec<AnimStep> {
    let frame = frame_of(anim, pivot, false);
    let rot_frame = if needs_matrix(anim, pivot) {
        frame_of(anim, pivot, true)
    } else {
        frame
    };
    // A translation along the X axis that a mesh-origin `origin_rot_y ±90` turns vertical (push
    // buttons, the sun blind) moves the other way than the right-handed rotation says: the
    // buttons' factors (`cockpit_gang4` 0.002 with -90, `cockpit_tuertaster1` -0.005 with +90)
    // press them *down* into the console, and the Bremen MAN's blind (`cp_rollo_pos` -1 with
    // factor 0.5, retracted at load) goes *up*. Rotations and explicit `origin_trans` frames keep
    // the convention (doors swing outward, wheels turn: checked).
    let flips_translation = anim.origin_from_mesh
        && anim.origin_trans.is_none()
        && anim.origin_rot.x == 0.0
        && anim.origin_rot.z == 0.0
        && (anim.origin_rot.y.abs() - 90.0).abs() < 1e-3;
    // The door chains turn the other way than the right-handed convention says, on the NLC's
    // y/z-swapped meshes and on the Bremen MAN's alike: with `door0 -177` the NLC's interior-swing
    // leaves swung out of the bus, the Bremen MAN's `door_0 75` leaves too (the top view of all
    // three door types shows it). The wheels of the same vehicles steer the right way, so this
    // is limited to the `door*` variables.
    let door = |a: &AnimAction| a.variable.to_ascii_lowercase().starts_with("door");
    // The scissor arms of the NLC's front sun blind (`Rollo_v`, `origin_rot_y -90` on y/z-swapped
    // meshes, arm angles 57 and 114 degrees) swung the fabric out of the bus, above the destination
    // board, with the right-handed sense: they turn like the door chains. The side blind, whose
    // meshes take their matrix rotation into the frame, keeps the convention.
    let swapped_arm = pivot.is_some_and(|p| p.is_swap_yz()) && !needs_matrix(anim, pivot);
    let blind_arm =
        |a: &AnimAction| swapped_arm && a.variable.to_ascii_lowercase().contains("rollo");
    // The sliding leaves of the NLC's outward-swinging sliding doors: `origin_rot_y 90` (the X axis
    // down) and `anim_trans door_0_schiebe 0.47` run along the bus on the swapped mesh.
    let swapped = swaps_translation(anim, pivot);
    let slide_frame = if swapped {
        Mat4::from_translation(frame.w_axis.truncate()) * swap_yz() * origin_rotation(anim)
    } else {
        frame
    };
    // A translation in the mesh's own frame is scaled like the mesh's matrix.
    let scale = pivot
        .filter(|p| anim.origin_from_mesh && anim.origin_trans.is_none() && p.scale > 0.0)
        .map_or(1.0, |p| p.scale);
    let mk = |a: &AnimAction, translate: bool| {
        let slides = translate && swapped && door(a);
        let a_factor = if translate {
            a.factor * scale
        } else {
            a.factor
        };
        AnimStep {
            frame: if slides {
                slide_frame
            } else if translate {
                frame
            } else {
                rot_frame
            },
            variable: a.variable.clone(),
            factor: if (translate && (flips_translation || slides))
                || (!translate && (door(a) || blind_arm(a)))
            {
                -a_factor
            } else {
                a_factor
            },
            translate,
            maxspeed: anim.maxspeed.filter(|v| *v > 0.0),
        }
    };
    // Translation first would move the rotation axis; rotation is the inner step.
    anim.anim_rot
        .iter()
        .map(|a| mk(a, false))
        .chain(anim.anim_trans.iter().map(|a| mk(a, true)))
        .collect()
}

/// The steps of one `[newanim]` block. `pivot` is the mesh file's own origin (the translation of
/// its matrix chunk); it is used for `origin_from_mesh` unless an explicit `origin_trans` is given.
/// The mesh's matrix rotation is not known here (identity); use [`anim_steps_with`] to pass it.
#[must_use]
pub fn anim_steps(anim: &ModelAnim, pivot: Option<Vec3>) -> Vec<AnimStep> {
    steps_of(anim, pivot.map(MeshPivot::plain))
}

/// [`anim_steps`] with the mesh's pivot and matrix rotation.
#[must_use]
pub fn anim_steps_with(anim: &ModelAnim, pivot: Option<MeshPivot>) -> Vec<AnimStep> {
    steps_of(anim, pivot)
}

/// Logs once per mesh that its steering/rotation axis was composed with the mesh matrix.
fn warn_matrix_once(mesh: &str) {
    use std::collections::HashSet;
    use std::sync::{Mutex, OnceLock};
    static SEEN: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    let seen = SEEN.get_or_init(Mutex::default);
    if seen.lock().is_ok_and(|mut s| s.insert(mesh.to_lowercase())) {
        tracing::warn!(
            mesh,
            "origin_from_mesh rotation axis composed with the mesh matrix rotation (animation-axes.md)"
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
mod tests;
