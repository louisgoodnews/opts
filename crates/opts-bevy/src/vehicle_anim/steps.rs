//! Turning the animation of a model into steps: the rotation, translation and matrix of each
//! frame.

use super::*;

/// The mesh matrix rotation to compose after the origin rotation of a rotation animation, if it
/// is needed (`docs/src/architecture/animation-axes.md`): the frame of `origin_from_mesh` is the
/// object's own. Only `origin_from_mesh` animations without `origin_trans` with an `anim_rot`
/// qualify, a mesh with a non-identity matrix, except
///
/// * on a y/z-swapped mesh, an axis the origin rotation already makes vertical (the 12 main
///   steering entries `origin_rot_y 90` on a swapped wheel, the NLC's factor-4 entry);
/// * a pure `origin_rot_x` on a proper rotation matrix (4 wheel spins on meshes with a yaw
///   matrix: they turn about x).
///
/// Census of the installed vehicles (the `anim_axis_probe` of 2026-10-26, 4,600 steering and
/// wheel-spin animations): every steering wheel turns about its measured rim normal and to the
/// right on a right turn, the 49 tilted ones of the Valiant, HOH and C2 families included
/// (`rot_x` 8.9 to 31.6 with `rot_z ±90`, none before); with the matrix the wheel spins of the
/// mirrored and turned wheel meshes roll forward (479 of 550, 112 without it).
pub(super) fn matrix_rotation(a: &ModelAnim, pivot: Option<MeshPivot>) -> Option<Mat4> {
    let pivot = pivot?;
    if !a.origin_from_mesh
        || a.origin_trans.is_some()
        || a.anim_rot.is_none()
        || pivot.is_identity()
    {
        return None;
    }
    let axis = origin_rotation(a).transform_vector3(Vec3::X);
    if pivot.is_swap_yz() {
        let tips_over = axis.z.abs() > 0.97;
        return (!tips_over).then(swap_yz);
    }
    let p = pivot.vehicle_rotation();
    let pure_x = a.origin_rot.x != 0.0 && a.origin_rot.y == 0.0 && a.origin_rot.z == 0.0;
    (!(pure_x && p.determinant() > 0.0)).then_some(p)
}

/// Whether a translation of a y/z-swapping mesh (the NLC) is taken along the swapped axis: the
/// sliding leaves of its outward-swinging sliding doors say `origin_rot_y 90` (the X axis down)
/// and `anim_trans door_0_schiebe 0.47`, which with the swap runs along the bus, not down.
pub(super) fn swaps_translation(a: &ModelAnim, pivot: Option<MeshPivot>) -> bool {
    pivot.is_some_and(|p| p.is_swap_yz())
        && a.origin_from_mesh
        && a.origin_trans.is_none()
        && a.anim_trans.is_some()
        && origin_rotation(a).transform_vector3(Vec3::X).z.abs() > 0.97
}

pub(super) fn needs_matrix(a: &ModelAnim, pivot: Option<MeshPivot>) -> bool {
    matrix_rotation(a, pivot).is_some()
}

/// The origin rotation in vehicle axes: OMSI's angles turn the other way here (one handedness).
pub(super) fn origin_rotation(a: &ModelAnim) -> Mat4 {
    Mat4::from_rotation_x(-a.origin_rot.x.to_radians())
        * Mat4::from_rotation_y(-a.origin_rot.y.to_radians())
        * Mat4::from_rotation_z(-a.origin_rot.z.to_radians())
}

pub(super) fn frame_of(a: &ModelAnim, pivot: Option<MeshPivot>, with_matrix: bool) -> Mat4 {
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

pub(super) fn steps_of(anim: &ModelAnim, pivot: Option<MeshPivot>) -> Vec<AnimStep> {
    let frame = frame_of(anim, pivot, false);
    let rot_frame = if needs_matrix(anim, pivot) {
        frame_of(anim, pivot, true)
    } else {
        frame
    };
    // With one handedness the push buttons (`cockpit_gang4` 0.002 on `origin_rot_y -90`) press
    // down into the console, the Bremen MAN's sun blind (`cp_rollo_pos` -1, factor 0.5) is
    // retracted up, the door leaves of the NLC and the Bremen MAN swing into and out of the bus as
    // their type says and the NLC's blind arms stay inside: until 2026-10-26 each of these had a
    // sign flip of its own on top of a right-handed origin rotation.
    // The sliding leaves of the NLC's outward-swinging sliding doors: `origin_rot_y 90` (the X axis
    // up) and `anim_trans door_0_schiebe 0.47` run along the bus on the swapped mesh.
    let door = |a: &AnimAction| a.variable.to_ascii_lowercase().starts_with("door");
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
            factor: a_factor,
            translate,
            maxspeed: anim.maxspeed.filter(|v| *v > 0.0),
            delay: anim.delay.filter(|v| *v > 0.0),
        }
    };
    // Translation first would move the rotation axis; rotation is the inner step.
    anim.anim_rot
        .iter()
        .map(|a| mk(a, false))
        .chain(anim.anim_trans.iter().map(|a| mk(a, true)))
        .collect()
}
