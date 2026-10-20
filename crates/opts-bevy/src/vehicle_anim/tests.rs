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

/// The 18C's steering blocks: the main one on `origin_rot_y 90`.
const NLC18: &str = "[mesh]\nsolo\\rad_vl_felge.o3d\n\n[newanim]\norigin_from_mesh\norigin_rot_y\n90\nanim_rot\nAxle_Steering_0_L\n57.29578\n\n[newanim]\norigin_from_mesh\norigin_rot_z\n90\nanim_rot\nAxle_Steering_0_L\n4\n\n++++++\n";

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
    let swapped = MeshPivot {
        origin: hub,
        rows: SWAP_YZ_ROWS,
    };
    let anim = steering_mesh(NLC, Some(swapped));
    assert!(steers_about_vertical(&anim.steps, hub));
    // Without the swap (no matrix) the same data rolls about the forward axis (the bug).
    let plain = MeshPivot::plain(hub);
    let anim = steering_mesh(NLC, Some(plain));
    assert!(!steers_about_vertical(&anim.steps, hub));
}

/// Where a point 1 m ahead of the hub goes (bevy x: right) when the steering variable is
/// negative, which is a right turn.
fn right_turn_x(cfg: &str, pivot: Option<MeshPivot>, hub: Vec3) -> f32 {
    let anim = steering_mesh(cfg, pivot);
    let m = animation_matrix(&anim.steps[..1], |_| -0.7);
    let p = bevy(hub + Vec3::Y);
    m.transform_point3(p).x - p.x
}

#[test]
fn a_negative_steering_variable_turns_the_wheels_right() {
    let hub = Vec3::new(-1.0, 3.0, 0.5);
    let swapped = Some(MeshPivot {
        origin: hub,
        rows: SWAP_YZ_ROWS,
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
    });
    let leaf = "[mesh]\nd.o3d\n\n[newanim]\norigin_from_mesh\norigin_rot_z\n90\nanim_rot\ndoor0\n-177\n\n[newanim]\norigin_from_mesh\norigin_rot_y\n90\nanim_trans\ndoor_0_schiebe\n0.47\n\n++++++\n";
    let anim = steering_mesh(leaf, swapped);
    let turn = anim.steps.iter().find(|s| !s.translate).expect("rotation");
    assert!(
        (turn.factor - 177.0).abs() < 1e-3,
        "door rotations are negated: {}",
        turn.factor
    );
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
    };
    let anim = steering_mesh(NLC, Some(swapped));
    // (factor 4: the variable is raised so that the angle is clearly measurable)
    let m = animation_matrix(&anim.steps[1..2], |_| 5.0);
    let p = bevy(Vec3::Y);
    let moved = m.transform_point3(p);
    assert!((moved.x - p.x).abs() > 0.1 && (moved.y - p.y).abs() < 1e-4);
}

/// The Bremen MAN's sun blind (`model_NG313_main.cfg`): the blind is retracted at load
/// (`cp_rollo_pos` -1), pulled down by `mouse_y / 500` per call towards 0.
const BLIND: &str = "[mesh]\nGeneric\\Bremen_LC_rollo_vorn.o3d\n\n[newanim]\norigin_from_mesh\norigin_rot_y\n-90\nanim_trans\ncp_rollo_pos\n0.5\n\n++++++\n";
/// A push button of the same class: pressed (`1`) it moves 2 mm into the console.
const BUTTON: &str = "[mesh]\nb.o3d\n\n[newanim]\norigin_from_mesh\norigin_rot_y\n-90\nanim_trans\ncockpit_gang4\n0.002\n\n++++++\n";

#[test]
fn a_retracted_blind_goes_up_and_a_pressed_button_goes_down() {
    let up = |cfg: &str, value: f32| {
        let anim = steering_mesh(cfg, Some(MeshPivot::plain(Vec3::ZERO)));
        let m = animation_matrix(&anim.steps, |_| value);
        // Bevy's y is up.
        m.transform_point3(Vec3::ZERO).y
    };
    assert!(
        up(BLIND, -1.0) > 0.4,
        "retracted: 0.5 m up, got {}",
        up(BLIND, -1.0)
    );
    assert!(
        up(BLIND, 0.0).abs() < 1e-6,
        "pulled down: where the mesh is"
    );
    assert!(up(BUTTON, 1.0) < -0.001, "pressed: into the console");
}

/// MAN_NL_NG `model_GN92_main.cfg` steering wheel: `origin_from_mesh`, no origin rotation.
const GN92_WHEEL: &str = "[mesh]\nGN92\\GN92_lenkrad.o3d\n\n[newanim]\norigin_from_mesh\nanim_rot\nAxle_Steering_0_L\n1680\n\n++++++\n";

/// The `GN92_Lenkrad.o3d` matrix rows (a 3-cycle with the column's 10° tilt) and the normal of
/// the rim measured from its descrambled vertices in vehicle axes (`anim_axis_geometry`).
const GN92_ROWS: [[f32; 3]; 3] = [
    [0.0, 0.984_815_8, -0.173_602_79],
    [0.0, -0.173_602_79, -0.984_815_8],
    [-1.0, 0.0, 0.0],
];
const GN92_RIM_NORMAL: Vec3 = Vec3::new(0.0, -0.177_297_74, 0.984_157_26);

#[test]
fn steering_wheel_turns_about_the_tilted_column_ng272() {
    let pivot = MeshPivot {
        origin: Vec3::new(-0.7, 5.2, 1.4),
        rows: GN92_ROWS,
    };
    let anim = steering_mesh(GN92_WHEEL, Some(pivot));
    let axis = anim.steps[0].frame.transform_vector3(Vec3::X);
    assert!(
        axis.dot(GN92_RIM_NORMAL).abs() > 0.99,
        "axis {axis:?} against the rim normal {GN92_RIM_NORMAL:?}"
    );
    // Without the matrix the wheel would turn about the lateral axis (the rim nodding up and down).
    let plain = steering_mesh(GN92_WHEEL, Some(MeshPivot::plain(pivot.origin)));
    let nod = plain.steps[0].frame.transform_vector3(Vec3::X);
    assert!(nod.dot(GN92_RIM_NORMAL).abs() < 0.1);
}
