//! Headlight and spotlight cones, and the ground clip height of a cone.

use super::*;

#[test]
fn a_headlight_cone_follows_the_light_and_a_spotlight_cone_its_range() {
    // OMSI forward (+y) is Bevy -Z.
    let h = light(Vec3::Y, false, false, 150.0, 190.0);
    let s = headlight_cone_shape(&h).expect("headlight");
    assert!((s.dir - Vec3::NEG_Z).length() < 1e-6);
    assert_eq!(s.length, HEADLIGHT_CONE_LENGTH_M);
    assert!(
        (s.half_angle - HEADLIGHT_CORONA_BEAM_HALF_DEG.to_radians()).abs() < 1e-6,
        "an outer angle of 190 degrees is a corona angle: a narrow beam"
    );
    let beam = headlight_cone_shape(&light(Vec3::Y, false, false, 20.0, 40.0)).expect("beam");
    assert!(
        (beam.half_angle - 20f32.to_radians()).abs() < 1e-6,
        "a real beam angle is kept"
    );
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
        (c.half_angle - HEADLIGHT_CORONA_BEAM_HALF_DEG.to_radians()).abs() < 1e-6,
        "35 degrees clamped to the haze limit"
    );
    let near = Spotlight {
        range: 12.0,
        ..spot
    };
    assert_eq!(spotlight_cone_shape(&near).length, 12.0);
    assert_eq!(c.transform().rotation, Quat::IDENTITY);
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
