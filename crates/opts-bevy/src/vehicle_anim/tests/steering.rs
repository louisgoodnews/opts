//! Steering: the vertical axis of steering blocks, the turn direction and the door leaves.

use super::*;

/// Excerpts of the steering blocks of MAN_SD200 `model_SD77.cfg` (explicit `origin_trans`) and
/// MAN_NewLionsCity `model_21C_4door_main.cfg` (`origin_from_mesh`, y/z-swap mesh matrix).
pub(super) const SD77: &str = "[mesh]\nwheel.o3d\n\n[newanim]\norigin_trans\n-0.95\n2.943\n0.495\norigin_rot_y\n90\nanim_rot\nAxle_Steering_0_L\n57.29578\n\n++++++\n";
const NLC: &str = "[mesh]\nsolo\\rad_vl_felge.o3d\n\n[newanim]\norigin_from_mesh\norigin_rot_z\n90\nanim_rot\nAxle_Steering_0_L\n57.29578\n\n[newanim]\norigin_from_mesh\norigin_rot_y\n90\nanim_rot\nAxle_Steering_0_L\n4\n\n++++++\n";

/// The 18C's steering blocks: the main one on `origin_rot_y 90`.
const NLC18: &str = "[mesh]\nsolo\\rad_vl_felge.o3d\n\n[newanim]\norigin_from_mesh\norigin_rot_y\n90\nanim_rot\nAxle_Steering_0_L\n57.29578\n\n[newanim]\norigin_from_mesh\norigin_rot_z\n90\nanim_rot\nAxle_Steering_0_L\n4\n\n++++++\n";


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
    let swapped = MeshPivot {
        origin: hub,
        rows: SWAP_YZ_ROWS,
        scale: 1.0,
    };
    let anim = steering_mesh(NLC, Some(swapped));
    assert!(steers_about_vertical(&anim.steps, hub));
    // Without the swap (no matrix) the same data rolls about the forward axis (the bug).
    let plain = MeshPivot::plain(hub);
    let anim = steering_mesh(NLC, Some(plain));
    assert!(!steers_about_vertical(&anim.steps, hub));
}

/// Where a point 1 m ahead of the hub goes (bevy x: right) when the steering variable is
/// positive, which is a right turn (the scripts' indicator cancel arms the right indicator on a
/// positive `Axle_Steering_0_L`).
pub(super) fn right_turn_x(cfg: &str, pivot: Option<MeshPivot>, hub: Vec3) -> f32 {
    let anim = steering_mesh(cfg, pivot);
    let m = animation_matrix(&anim.steps[..1], |_| 0.7);
    let p = bevy(hub + Vec3::Y);
    m.transform_point3(p).x - p.x
}

#[test]
fn a_positive_steering_variable_turns_the_wheels_right() {
    let hub = Vec3::new(-1.0, 3.0, 0.5);
    let swapped = Some(MeshPivot {
        origin: hub,
        rows: SWAP_YZ_ROWS,
        scale: 1.0,
    });
    let nlc21 = right_turn_x(NLC, swapped, hub);
    let nlc18 = right_turn_x(NLC18, swapped, hub);
    let sd77 = right_turn_x(SD77, None, Vec3::new(-0.95, 2.943, 0.495));
    assert!(sd77 > 0.1, "SD77 {sd77}");
    assert!(nlc21 > 0.1, "NLC (rot_z) {nlc21}");
    assert!(nlc18 > 0.1, "NLC (rot_y) {nlc18}");
}

#[test]
fn door_leaves_turn_the_other_way_and_the_sliding_leaf_runs_along_the_bus() {
    let swapped = Some(MeshPivot {
        origin: Vec3::ZERO,
        rows: SWAP_YZ_ROWS,
        scale: 1.0,
    });
    let leaf = "[mesh]\nd.o3d\n\n[newanim]\norigin_from_mesh\norigin_rot_z\n90\nanim_rot\ndoor0\n-177\n\n[newanim]\norigin_from_mesh\norigin_rot_y\n90\nanim_trans\ndoor_0_schiebe\n0.47\n\n++++++\n";
    let anim = steering_mesh(leaf, swapped);
    let turn = anim.steps.iter().find(|s| !s.translate).expect("rotation");
    // Opening (`door0` growing) turns the leaf anticlockwise seen from above (the sense the top
    // view of the NLC's interior-swing door showed right): a point ahead of the hinge goes left.
    let m = animation_matrix(std::slice::from_ref(turn), |_| 0.1);
    let ahead = m.transform_point3(bevy(Vec3::Y));
    assert!(ahead.x < -0.2 && (ahead.y).abs() < 1e-4, "{ahead}");
    let slide = anim.steps.iter().find(|s| s.translate).expect("slide");
    let m = animation_matrix(std::slice::from_ref(slide), |_| 1.0);
    let moved = m.transform_point3(Vec3::ZERO);
    // Bevy: z is the bus's length, y up.
    assert!(
        moved.y.abs() < 1e-4 && (moved.z.abs() - 0.47).abs() < 1e-3,
        "the leaf slides along the bus: {moved}"
    );
}

#[test]
fn the_matrix_is_left_out_where_the_axis_is_already_vertical() {
    // The class `origin_rot_y 90` on a swapped mesh is not changed (the side-effect entry).
    let swapped = MeshPivot {
        origin: Vec3::ZERO,
        rows: SWAP_YZ_ROWS,
        scale: 1.0,
    };
    let anim = steering_mesh(NLC, Some(swapped));
    // (factor 4: the variable is raised so that the angle is clearly measurable)
    let m = animation_matrix(&anim.steps[1..2], |_| 5.0);
    let p = bevy(Vec3::Y);
    let moved = m.transform_point3(p);
    assert!((moved.x - p.x).abs() > 0.1 && (moved.y - p.y).abs() < 1e-4);
}
