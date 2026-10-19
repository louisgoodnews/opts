//! Cone mapping: position, angles, tag, ground clip.

use super::*;

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

#[test]
fn a_headlight_cone_follows_the_light_and_a_spotlight_cone_its_range() {
    // OMSI forward (+y) is Bevy -Z.
    let h = light(Vec3::Y, false, false, 150.0, 190.0);
    let s = headlight_cone_shape(&h).expect("headlight");
    assert!((s.dir - Vec3::NEG_Z).length() < 1e-6);
    assert_eq!(s.length, HEADLIGHT_CONE_LENGTH_M);
    assert!((s.half_angle - 25f32.to_radians()).abs() < 1e-6);
    assert!(headlight_cone_shape(&light(Vec3::ZERO, false, false, 1.0, 2.0)).is_none());

    let spot = Spotlight {
        pos: Vec3::new(0.5, 3.0, 0.7),
        dir: Vec3::Y,
        color_rgb: [255, 245, 215],
        range: 500.0,
        inner_cone: 30.0,
        outer_cone: 70.0,
    };
    let c = spotlight_cone_shape(&spot);
    // In the SpotLight entity's frame: apex at the origin, along -Z, range capped at 40 m.
    assert_eq!((c.apex, c.dir), (Vec3::ZERO, Vec3::NEG_Z));
    assert_eq!(c.length, SPOT_CONE_MAX_LENGTH_M);
    assert!(
        (c.half_angle - 25f32.to_radians()).abs() < 1e-6,
        "35 degrees clamped"
    );
    let near = Spotlight {
        range: 12.0,
        ..spot
    };
    assert_eq!(spotlight_cone_shape(&near).length, 12.0);
    assert_eq!(c.transform().rotation, Quat::IDENTITY);
}

#[test]
fn the_tag_packs_colour_level_inner_edge_and_ground_height() {
    let tag = pack_tag([1.0, 0.5, 0.0], 0.5, 0.2, Some(7.0));
    assert_eq!(tag & 31, 31);
    assert_eq!((tag >> 5) & 31, 16);
    assert_eq!((tag >> 10) & 31, 0);
    assert_eq!(tag_level(tag), 32);
    assert_eq!((tag >> 21) & 15, 3);
    assert_eq!(tag_ground_h(tag), Some(7.0));
    assert_eq!(tag_ground_h(pack_tag([0.0; 3], 1.0, 0.0, None)), None);
    assert_eq!(
        tag_level(pack_tag([0.0; 3], 2.0, 0.0, None)),
        63,
        "level is clamped"
    );
    assert_eq!(tag_level(pack_tag([0.0; 3], -1.0, 0.0, None)), 0);
    assert_eq!(
        tag_ground_h(pack_tag([0.0; 3], 1.0, 0.0, Some(99.0))),
        Some(31.5)
    );
}

#[test]
fn the_ground_clip_height_follows_the_apex() {
    let down = Vec3::new(0.0, 0.0, -1.0);
    let s = lantern_cone_shape(&light(down, false, true, 120.0, 200.0)).expect("cone");
    assert_eq!(s.ground_h, Some(7.0));
    let h = headlight_cone_shape(&light(Vec3::Y, false, false, 50.0, 190.0)).expect("head");
    assert_eq!(h.ground_h, Some(7.0), "the fixture's z is 7 m");
    let mut low = light(Vec3::Y, false, false, 50.0, 190.0);
    low.position = Vec3::new(0.5, 1.9, 0.1);
    assert_eq!(headlight_cone_shape(&low).unwrap().ground_h, None);
    let spot = Spotlight {
        pos: Vec3::new(0.5, 3.0, 0.7),
        dir: Vec3::Y,
        color_rgb: [255; 3],
        range: 100.0,
        inner_cone: 30.0,
        outer_cone: 70.0,
    };
    assert_eq!(spotlight_cone_shape(&spot).ground_h, Some(0.7));
}

#[test]
fn the_prepass_gate_waits_then_fades_in() {
    assert_eq!(prepass_gate(false, 0.0, 0), 1.0, "no fade, no gate");
    assert_eq!(prepass_gate(true, 0.0, 0), 0.0);
    assert_eq!(prepass_gate(true, PREPASS_READY_S - 0.1, 100), 0.0);
    assert_eq!(
        prepass_gate(true, PREPASS_READY_S + 5.0, 2),
        0.0,
        "frames too"
    );
    let mid = prepass_gate(true, PREPASS_READY_S + CONE_FADE_IN_S * 0.5, 100);
    assert!((mid - 0.5).abs() < 1e-5);
    assert_eq!(prepass_gate(true, PREPASS_READY_S + 10.0, 1000), 1.0);
}
