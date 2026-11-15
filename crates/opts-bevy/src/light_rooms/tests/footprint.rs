//! The vehicle footprint: the over-the-roof test and the distance to the footprint.

use super::*;

#[test]
fn a_lamp_over_the_roof_is_over_the_vehicle() {
    let fp = footprint();
    let at = |x, y, z| {
        over_vehicle(
            Vec3::new(x, y, z),
            &GlobalTransform::default(),
            &fp,
            FOOTPRINT_MARGIN_M,
        )
    };
    assert!(at(0.0, 7.0, 0.0), "straight above");
    assert!(at(2.0, 7.0, 6.5), "within the margin");
    assert!(!at(2.6, 7.0, 0.0), "beside the vehicle");
    assert!(!at(0.0, 7.0, 8.0), "ahead of it");
    assert!(!at(0.0, 2.0, 0.0), "below the roof");
    // The vehicle turned 90 degrees: its length now lies along X.
    let turned = GlobalTransform::from(Transform::from_rotation(Quat::from_rotation_y(
        std::f32::consts::FRAC_PI_2,
    )));
    assert!(over_vehicle(Vec3::new(5.0, 7.0, 0.0), &turned, &fp, 1.0));
    assert!(!over_vehicle(Vec3::new(0.0, 7.0, 5.0), &turned, &fp, 1.0));
}

#[test]
fn the_distance_to_the_footprint_is_zero_inside() {
    let fp = footprint();
    assert_eq!(fp.distance(Vec3::new(0.0, 1.0, 0.0)), 0.0);
    assert!((fp.distance(Vec3::new(4.3, 1.0, 0.0)) - 3.0).abs() < 1e-5);
    assert!((fp.distance(Vec3::new(1.3, 3.2 + 4.0, 6.0 + 3.0)) - 5.0).abs() < 1e-5);
}
