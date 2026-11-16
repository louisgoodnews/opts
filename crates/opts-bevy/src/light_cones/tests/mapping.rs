//! Cone mapping: position, angles, tag, ground clip.

use super::*;

mod gate;
mod headlight;
mod tag;

#[test]
fn only_directional_lights_with_the_cone_flag_get_a_lantern_cone() {
    let down = Vec3::new(0.0, 0.0, -1.0);
    assert!(lantern_cone_shape(&light(down, false, true, 120.0, 200.0)).is_some());
    assert!(
        lantern_cone_shape(&light(down, true, true, 120.0, 200.0)).is_none(),
        "omni"
    );
    assert!(
        lantern_cone_shape(&light(down, false, false, 20.0, 40.0)).is_none(),
        "a narrow signal lamp without the flag"
    );
    assert!(
        lantern_cone_shape(&light(Vec3::ZERO, false, true, 20.0, 40.0)).is_none(),
        "no direction"
    );
}

#[test]
fn a_lantern_cone_maps_position_direction_angles_length_and_colour() {
    // OMSI (1, 2, 7) is Bevy (1, 7, -2); OMSI down (0, 0, -1) is Bevy (0, -1, 0).
    let s = lantern_cone_shape(&light(Vec3::new(0.0, 0.0, -1.0), false, true, 120.0, 200.0))
        .expect("cone");
    assert_eq!(s.apex, Vec3::new(1.0, 7.0, -2.0));
    assert!((s.dir - Vec3::NEG_Y).length() < 1e-6);
    // 200 degrees is clamped to the 40 degree half angle; the inner edge keeps the 120/200 ratio.
    assert!((s.half_angle - 40f32.to_radians()).abs() < 1e-6);
    let want = (40f32.to_radians() * 0.6).tan() / 40f32.to_radians().tan();
    assert!((s.inner_frac - want).abs() < 1e-5, "{}", s.inner_frac);
    // Pointing down from 7 m: the length is the way to the ground.
    assert!((s.length - 7.0).abs() < 1e-5);
    assert_eq!(s.color, [1.0, 128.0 / 255.0, 0.0]);
    // The transform maps the unit cone's -Z onto the direction and scales radius and length.
    let t = s.transform();
    assert_eq!(t.translation, s.apex);
    assert!((t.rotation * Vec3::NEG_Z - Vec3::NEG_Y).length() < 1e-5);
    assert!((t.scale.z - 7.0).abs() < 1e-5);
    assert!((t.scale.x - 7.0 * 40f32.to_radians().tan()).abs() < 1e-4);
}

#[test]
fn lantern_length_falls_back_when_it_does_not_point_down() {
    assert_eq!(
        lantern_cone_length(Vec3::new(0.0, 7.0, 0.0), Vec3::X),
        LANTERN_DEFAULT_LENGTH_M
    );
    // Slanted: the way along the axis to the ground, clamped.
    let d = Vec3::new(1.0, -1.0, 0.0).normalize();
    assert!((lantern_cone_length(Vec3::new(0.0, 4.0, 0.0), d) - 4.0 * 2f32.sqrt()).abs() < 1e-4);
    assert_eq!(
        lantern_cone_length(Vec3::new(0.0, 100.0, 0.0), Vec3::NEG_Y),
        LANTERN_LENGTH_M.1
    );
    assert_eq!(
        lantern_cone_length(Vec3::new(0.0, 0.5, 0.0), Vec3::NEG_Y),
        LANTERN_LENGTH_M.0
    );
}

#[test]
fn cone_angles_keep_the_ratio_and_the_limits() {
    let (half, inner) = cone_angles(12.0, 30.0, HEADLIGHT_HALF_DEG);
    assert!((half - 15f32.to_radians()).abs() < 1e-6);
    let want = (15f32.to_radians() * 0.4).tan() / 15f32.to_radians().tan();
    assert!((inner - want).abs() < 1e-5);
    // Too narrow and too wide are clamped; no outer angle gives the widest.
    assert!((cone_angles(0.0, 4.0, HEADLIGHT_HALF_DEG).0 - 6f32.to_radians()).abs() < 1e-6);
    assert!((cone_angles(0.0, 190.0, HEADLIGHT_HALF_DEG).0 - 25f32.to_radians()).abs() < 1e-6);
    assert!((cone_angles(0.0, 0.0, HEADLIGHT_HALF_DEG).0 - 25f32.to_radians()).abs() < 1e-6);
    // Inner larger than outer is held below the outer edge.
    assert!(cone_angles(80.0, 30.0, HEADLIGHT_HALF_DEG).1 <= 0.95);
}
