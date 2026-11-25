//! Scenery `origin_from_mesh` animations (barrier arms, ISS-059 and ISS-060) use the mesh's own
//! pivot with its matrix rotation, as the vehicle rule does, not the bare origin.

use bevy::math::Vec3;
use opts_bevy::{anim_steps, anim_steps_with, animation_matrix, AnimStep, MeshPivot};
use opts_core::formats::vehicles::model::ModelConfig;

/// A barrier arm that turns about a horizontal axis (`origin_from_mesh`, `origin_rot_z 90`).
/// A rotation whose axis tips over on a swapped mesh is not composed with the matrix (vehicle rule).
const ARM: &str = "[mesh]\nHs64_Schranke_R.o3d\n\n[newanim]\norigin_from_mesh\norigin_rot_z\n90\nanim_rot\nbarrier\n60\n\n++++++\n";

/// The rotation rows of a y/z swapped matrix chunk (the modelling tool's axes of the arm's file).
const SWAP_ROWS: [[f32; 3]; 3] = [[1.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, 1.0, 0.0]];

/// The point of the arm moved by its animation at value 1 (Bevy axes, as the mesh vertices are).
fn moved(steps: &[AnimStep], p: Vec3) -> Vec3 {
    animation_matrix(steps, |_| 1.0).transform_point3(p)
}

#[test]
fn from_o3d_transform_reads_origin_in_vehicle_axes_and_normalises_rows() {
    // o3d axes: the translation row is (x, y, z) = (1, 2, 3); the vehicle origin swaps y and z.
    let mut m = [[0.0_f32; 4]; 4];
    m[0] = [0.01, 0.0, 0.0, 0.0];
    m[1] = [0.0, 0.0, 0.01, 0.0];
    m[2] = [0.0, 0.01, 0.0, 0.0];
    m[3] = [1.0, 2.0, 3.0, 1.0];
    let pivot = MeshPivot::from_o3d_transform(m);
    assert_eq!(pivot.origin, Vec3::new(1.0, 3.0, 2.0));
    assert!((pivot.scale - 0.01).abs() < 1e-6, "{pivot:?}");
    for (row, expected) in pivot.rows.iter().zip(SWAP_ROWS) {
        for (a, b) in row.iter().zip(expected) {
            assert!((a - b).abs() < 1e-6, "{pivot:?}");
        }
    }
}

#[test]
fn a_plain_matrix_pivot_gives_the_same_motion_as_the_bare_origin() {
    // Regression guard: with no rotation in the matrix the new path must not change the motion.
    let origin = Vec3::new(0.2, 1.1, -0.4);
    let model = ModelConfig::parse(ARM).expect("parses");
    let anim = &model.lods[0].meshes[0].anims[0];
    let old = anim_steps(anim, Some(origin));
    let new = anim_steps_with(anim, Some(MeshPivot::plain(origin)));
    assert_eq!(old.len(), new.len());
    let p = Vec3::new(0.0, 0.0, 1.5);
    assert!(moved(&old, p).abs_diff_eq(moved(&new, p), 1e-5));
}

#[test]
fn a_swapped_matrix_turns_the_arm_about_the_frame_of_its_file() {
    // The arm's file matrix swaps y and z: the arm turns about the axis its file gives, so its
    // tip ends up somewhere else than with the bare origin (which has no rotation of its own).
    let origin = Vec3::new(0.0, 0.5, 0.0);
    let model = ModelConfig::parse(ARM).expect("parses");
    let anim = &model.lods[0].meshes[0].anims[0];
    let bare = anim_steps(anim, Some(origin));
    let swapped = MeshPivot {
        rows: SWAP_ROWS,
        ..MeshPivot::plain(origin)
    };
    let with_matrix = anim_steps_with(anim, Some(swapped));
    let tip = Vec3::new(0.0, 0.0, 2.0);
    let (a, b) = (moved(&bare, tip), moved(&with_matrix, tip));
    assert!(!a.abs_diff_eq(b, 1e-3), "bare {a:?} vs matrix {b:?}");
}
