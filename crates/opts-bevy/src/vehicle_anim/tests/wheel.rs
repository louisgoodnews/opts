//! Steering wheels and the column: the rim turns about its tilted column, with the road wheels.

use super::*;

use super::steering::{right_turn_x, SD77};

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
