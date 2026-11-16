//! Mirror reflection, camera pose and field of view.

use super::*;

#[test]
fn reflection_flips_the_normal_component_and_keeps_the_rest() {
    let d = Vec3::new(1.0, -1.0, 0.0).normalize();
    let r = reflect(d, Vec3::Y);
    assert!(close(r, Vec3::new(1.0, 1.0, 0.0).normalize()));
    assert!((r.length() - 1.0).abs() < 1e-5, "length kept");
    assert!(
        close(reflect(r, Vec3::Y), d),
        "reflecting twice is the identity"
    );
    // Angle of incidence equals angle of reflection.
    assert!((d.dot(Vec3::Y) + r.dot(Vec3::Y)).abs() < 1e-5);
    // A ray along the plane is untouched, a ray along the normal turns around.
    assert!(close(reflect(Vec3::X, Vec3::Y), Vec3::X));
    assert!(close(reflect(Vec3::Y, Vec3::Y), -Vec3::Y));
}

#[test]
fn the_normal_is_turned_to_face_the_viewer() {
    let view = Vec3::NEG_Z;
    assert_eq!(facing_normal(Vec3::Z, view), Vec3::Z);
    assert_eq!(facing_normal(Vec3::NEG_Z, view), Vec3::Z);
    assert!(view.dot(facing_normal(Vec3::new(0.3, 0.1, -0.9).normalize(), view)) <= 0.0);
}

#[test]
fn a_viewer_straight_in_front_sees_straight_back() {
    let (pos, dir) = mirror_camera_pose(Vec3::ZERO, Vec3::new(0.0, 0.0, -5.0), Vec3::Z).unwrap();
    assert!(close(dir, Vec3::Z), "{dir:?}");
    assert!(close(pos, Vec3::new(0.0, 0.0, -5.0 + MIRROR_OFFSET_M)));
}

#[test]
fn the_camera_looks_along_the_line_from_the_mirrored_viewer_through_the_mirror() {
    let (viewer, centre) = (Vec3::new(3.0, 1.5, 2.0), Vec3::new(0.5, 1.2, -4.0));
    let normal = Vec3::new(0.2, 0.0, 1.0).normalize();
    let (pos, dir) = mirror_camera_pose(viewer, centre, normal).unwrap();
    // The virtual viewer behind the mirror plane.
    let virtual_viewer = viewer - 2.0 * (viewer - centre).dot(normal) * normal;
    assert!(close((centre - virtual_viewer).normalize(), dir));
    assert!(close(dir, reflect((centre - viewer).normalize(), normal)));
    // On the viewer's side of the mirror plane.
    assert!((pos - centre).dot(normal) > 0.0);
    // The flipped normal gives the same camera.
    let (pos2, dir2) = mirror_camera_pose(viewer, centre, -normal).unwrap();
    assert!(close(pos, pos2) && close(dir, dir2));
}

#[test]
fn a_degenerate_mirror_has_no_pose() {
    assert!(mirror_camera_pose(Vec3::ONE, Vec3::ONE, Vec3::Z).is_none());
    assert!(mirror_camera_pose(Vec3::ZERO, Vec3::X, Vec3::ZERO).is_none());
}

#[test]
fn the_field_of_view_follows_the_size_and_stays_in_range() {
    let near = mirror_fov(1.0, 1.5);
    let far = mirror_fov(1.0, 2.5);
    assert!(near > far, "a closer mirror covers more of the view");
    assert!((near - 2.0 * (1.0f32 / 3.0).atan()).abs() < 1e-5);
    let (lo, hi) = MIRROR_FOV_RANGE_DEG;
    assert!((mirror_fov(0.01, 20.0) - lo.to_radians()).abs() < 1e-6);
    assert!((mirror_fov(5.0, 0.1) - hi.to_radians()).abs() < 1e-6);
}
