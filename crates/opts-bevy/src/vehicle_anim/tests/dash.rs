//! Dash fittings: the sun blind, the push button and the speedometer needle.

use super::*;

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
