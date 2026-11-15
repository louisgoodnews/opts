//! Motion of the wheels and suspension: rolling, suspension travel, step order and the rate limit.

use super::*;

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
        delay: None,
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
        delay: None,
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

#[test]
fn a_translation_in_the_mesh_frame_is_scaled_like_the_mesh_matrix() {
    // The MB C2 BVG's gear button: `origin_from_mesh`, `anim_trans cockpit_gang3 0.8`, a matrix
    // of scale about 0.0094: pressed it moves 7.5 mm, not 0.8 m (corpus: the O530's buttons 0.6,
    // the DL05's horn -0.1, all on meshes scaled about 0.01).
    let cfg = "[mesh]\nZF_D.o3d\n[newanim]\norigin_from_mesh\nanim_trans\ncockpit_gang3\n0.8\n";
    let pivot = MeshPivot {
        scale: 0.0094,
        ..MeshPivot::plain(Vec3::new(-0.4, 5.47, 1.41))
    };
    let anim = steering_mesh(cfg, Some(pivot));
    let moved = animation_matrix_from_values(&anim.steps, &[1.0]).transform_point3(Vec3::ZERO);
    assert!((moved.length() - 0.8 * 0.0094).abs() < 1e-4, "{moved:?}");
    // An explicit `origin_trans` frame is the vehicle's: metres.
    let explicit = "[mesh]\nx.o3d\n[newanim]\norigin_trans\n0\n0\n0\nanim_trans\nv\n0.8\n";
    let anim = steering_mesh(explicit, Some(pivot));
    let moved = animation_matrix_from_values(&anim.steps, &[1.0]).transform_point3(Vec3::ZERO);
    assert!((moved.length() - 0.8).abs() < 1e-4, "{moved:?}");
}

