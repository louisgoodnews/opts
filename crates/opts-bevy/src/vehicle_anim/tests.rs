use super::*;

fn spin(var: &str) -> AnimStep {
    AnimStep {
        frame: Mat4::from_translation(Vec3::new(1.2, 2.9, 0.5)),
        variable: var.into(),
        factor: 57.295_78,
        translate: false,
        maxspeed: None,
        delay: None,
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
fn right_turn_x(cfg: &str, pivot: Option<MeshPivot>, hub: Vec3) -> f32 {
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
        scale: 1.0,
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

/// Whether a positive steering variable turns a steering wheel with the rim normal `normal`
/// clockwise as the driver sees it (looking down the column): a point on the rim moves to the
/// right of where it was.
fn turns_clockwise(step: &AnimStep, normal: Vec3) -> bool {
    let n = if normal.z < 0.0 { -normal } else { normal };
    let u = (Vec3::Y - n * Vec3::Y.dot(n)).normalize();
    let m = animation_matrix_from_values(std::slice::from_ref(step), &[0.02]);
    let c = bevy(Vec3::ZERO);
    let moved = m.transform_point3(bevy(u)) - m.transform_point3(c);
    // Back to OMSI axes.
    let moved = Vec3::new(moved.x, -moved.z, moved.y);
    u.cross(moved).dot(n) < 0.0
}

/// Valiant 7700 `Model_7700A_3p_main.cfg`: `origin_from_mesh`, `origin_rot_x 30.7`,
/// `origin_rot_z 90`, factor 1575, on `7700_DASH_BASE_STEERING_WHEEL.o3d` (a y/z-swap matrix of
/// scale 0.048; the rim normal measured from its vertices in vehicle axes).
const VALIANT_WHEEL: &str = "[mesh]\nDASH\\7700_DASH_BASE_STEERING_WHEEL.o3d\n\n[newanim]\norigin_from_mesh\norigin_rot_x\n30.7\norigin_rot_y\n0\norigin_rot_z\n90\nanim_rot\nAxle_Steering_0_L\n1575\n\n++++++\n";
const VALIANT_RIM_NORMAL: Vec3 = Vec3::new(-0.001_5, -0.487_5, 0.873_1);

#[test]
fn the_tilted_steering_wheel_turns_about_its_column_valiant() {
    let pivot = MeshPivot {
        origin: Vec3::new(-0.71, 5.19, 1.33),
        rows: SWAP_YZ_ROWS,
        scale: 0.048,
    };
    let anim = steering_mesh(VALIANT_WHEEL, Some(pivot));
    let axis = anim.steps[0].frame.transform_vector3(Vec3::X).normalize();
    assert!(
        axis.dot(VALIANT_RIM_NORMAL).abs() > 0.99,
        "axis {axis} against the rim normal {VALIANT_RIM_NORMAL}"
    );
    assert!(turns_clockwise(&anim.steps[0], VALIANT_RIM_NORMAL));
}

/// MAN_SD200 `model_SD77.cfg` steering wheel: explicit `origin_trans`, `origin_rot_x -10`,
/// `origin_rot_y 90`, factor 1450; `SD77_lenkrad.o3d` (identity matrix) has the rim normal
/// (0, -0.174, 0.985): the column leans back towards the driver.
const SD77_WHEEL: &str = "[mesh]\nSD77\\SD77_lenkrad.o3d\n\n[newanim]\norigin_trans\n-0.72\n5.167\n1.733\norigin_rot_x\n-10\norigin_rot_y\n90\norigin_rot_z\n0\nanim_rot\nAxle_Steering_0_L\n1450\n\n++++++\n";

#[test]
fn the_column_of_the_sd77_leans_towards_the_driver() {
    let normal = Vec3::new(0.0, -0.174_0, 0.984_7);
    let anim = steering_mesh(SD77_WHEEL, None);
    let axis = anim.steps[0].frame.transform_vector3(Vec3::X).normalize();
    assert!(axis.dot(normal).abs() > 0.999, "axis {axis}");
    assert!(turns_clockwise(&anim.steps[0], normal));
}

#[test]
fn steering_wheels_turn_with_the_road_wheels() {
    let pivot = MeshPivot {
        origin: Vec3::new(-0.7, 5.2, 1.4),
        rows: GN92_ROWS,
        scale: 1.0,
    };
    let gn92 = steering_mesh(GN92_WHEEL, Some(pivot));
    assert!(turns_clockwise(&gn92.steps[0], GN92_RIM_NORMAL));
    // The road wheels of the same sign turn right (`a_positive_steering_variable_...`).
    assert!(right_turn_x(SD77, None, Vec3::new(-0.95, 2.943, 0.495)) > 0.1);
}

/// Valiant 7700 speedometer needle: `origin_from_mesh`, `origin_rot_x 52`, `origin_rot_z 90`,
/// `anim_rot cockpit_tachowinkel 1` on `7700_DASH_NEEDLE_SPEED.o3d` (a scaled y/z swap); the dial
/// faces the driver, up and back (measured from the needle's vertices, about 4 degrees off).
const VALIANT_NEEDLE: &str = "[mesh]\nDASH\\7700_DASH_NEEDLE_SPEED.o3d\n\n[newanim]\norigin_from_mesh\norigin_rot_x\n52\norigin_rot_y\n0\norigin_rot_z\n90\nanim_rot\ncockpit_tachowinkel\n1\n\n++++++\n";

#[test]
fn the_needles_turn_on_the_dial_valiant() {
    let pivot = MeshPivot {
        origin: Vec3::new(-0.64, 5.54, 1.34),
        rows: SWAP_YZ_ROWS,
        scale: 0.01,
    };
    let dial = Vec3::new(0.0, -0.738, 0.675);
    let anim = steering_mesh(VALIANT_NEEDLE, Some(pivot));
    let axis = anim.steps[0].frame.transform_vector3(Vec3::X).normalize();
    assert!(
        axis.dot(dial).abs() > 0.99,
        "axis {axis} against the dial {dial}"
    );
    // A growing angle turns the needle clockwise as the driver sees the dial.
    let mut step = anim.steps[0].clone();
    step.factor = 1000.0;
    assert!(turns_clockwise(&step, dial));
}

fn damped(rate: f32, target: f32, dt: f32, seconds: f32) -> f32 {
    let mut step = spin("needle");
    step.delay = Some(rate);
    let mut anim = MeshAnimation {
        owner: Entity::PLACEHOLDER,
        steps: vec![step],
        visible: Vec::new(),
        shown: Vec::new(),
    };
    anim.follow(&[0.0], dt);
    let frames = (seconds / dt).round() as usize;
    for _ in 0..frames {
        anim.follow(&[target], dt);
    }
    anim.shown[0]
}

#[test]
fn delay_step_response_is_independent_of_the_frame_rate() {
    // A needle with delay 4 /s after 0.5 s: 1 - e^-2 of the step.
    let expect = 10.0 * (1.0 - (-2.0f32).exp());
    for dt in [1.0 / 144.0, 1.0 / 60.0, 1.0 / 30.0, 0.125, 0.5] {
        let v = damped(4.0, 10.0, dt, 0.5);
        assert!((v - expect).abs() < 1e-3, "dt {dt}: {v} vs {expect}");
    }
}

#[test]
fn delay_starts_at_the_first_value_and_settles_without_overshoot() {
    let mut step = spin("needle");
    step.delay = Some(2.0);
    let mut anim = MeshAnimation {
        owner: Entity::PLACEHOLDER,
        steps: vec![step],
        visible: Vec::new(),
        shown: Vec::new(),
    };
    anim.follow(&[7.0], 1.0 / 60.0);
    assert_eq!(anim.shown, [7.0], "no ramp from zero at spawn");
    for _ in 0..2000 {
        anim.follow(&[3.0], 1.0 / 60.0);
        assert!(anim.shown[0] >= 3.0 && anim.shown[0] <= 7.0);
    }
    assert!((anim.shown[0] - 3.0).abs() < 1e-3);
    // A huge frame (a stall) lands on the target, never beyond it.
    anim.follow(&[9.0], 100.0);
    assert!((anim.shown[0] - 9.0).abs() < 1e-4);
}

#[test]
fn no_delay_is_byte_identical_to_the_undamped_follow() {
    let step = spin("w");
    for (shown, target, dt) in [(0.0f32, 1.0f32, 0.016f32), (3.5, -2.0, 0.5), (1.0, 1.0, 0.0)] {
        assert_eq!(step.follow(shown, target, dt).to_bits(), target.to_bits());
    }
    let mut slow = spin("w");
    slow.maxspeed = Some(0.5);
    assert_eq!(slow.follow(0.0, 1.0, 1.0), 0.5);
}

#[test]
fn delay_with_maxspeed_is_limited_by_the_slower_of_the_two() {
    let mut step = spin("w");
    step.delay = Some(50.0);
    step.maxspeed = Some(0.5);
    let v = step.follow(0.0, 1.0, 0.1);
    assert!((v - 0.05).abs() < 1e-6, "{v}");
}
