//! Pure geometry: reflection, pose, field of view, pane and UV.

use super::*;

pub(super) fn close(a: Vec3, b: Vec3) -> bool {
    (a - b).length() < 1e-4
}

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

#[test]
fn only_mirrors_in_front_of_the_camera_count() {
    let f = Vec3::NEG_Z;
    assert!(mirror_in_front(Vec3::ZERO, f, Vec3::new(0.0, 0.0, -3.0)));
    assert!(mirror_in_front(Vec3::ZERO, f, Vec3::new(2.0, 0.0, -3.0)));
    assert!(
        !mirror_in_front(Vec3::ZERO, f, Vec3::new(0.0, 0.0, 3.0)),
        "behind"
    );
    assert!(
        !mirror_in_front(Vec3::ZERO, f, Vec3::new(5.0, 0.0, -0.5)),
        "far to the side"
    );
    assert!(!mirror_in_front(Vec3::ONE, f, Vec3::ONE), "at the camera");
}

#[test]
fn the_turn_pattern_is_every_nth_frame_and_staggered() {
    for every in [1u8, 2, 4] {
        let count = |rank| (0..8).filter(|f| is_render_frame(*f, rank, every)).count();
        assert_eq!(count(0), 8 / usize::from(every), "every {every}");
        assert_eq!(count(1), 8 / usize::from(every));
    }
    assert!((0..8).all(|f| is_render_frame(f, 0, 1)));
    assert_ne!(
        is_render_frame(0, 0, 2),
        is_render_frame(0, 1, 2),
        "staggered"
    );
}

#[test]
fn the_nearest_mirrors_are_picked_first() {
    assert_eq!(nearest(&[3.0, 1.0, 2.0], 2), vec![1, 2]);
    assert_eq!(nearest(&[3.0, 1.0, 2.0], 0), Vec::<usize>::new());
    assert_eq!(nearest(&[3.0, 1.0], 5), vec![1, 0]);
    assert_eq!(nearest(&[1.0, 1.0], 1), vec![0], "ties by index");
    assert_eq!(
        nearest(&[f32::INFINITY, 2.0], 2),
        vec![1],
        "no visible pane, never picked"
    );
}

#[test]
fn the_geometry_of_a_pane_is_its_centre_normal_height_and_aspect() {
    let pos = [
        [0.0, 0.0, 0.0],
        [0.4, 0.0, 0.0],
        [0.4, 0.2, 0.0],
        [0.0, 0.2, 0.0],
    ];
    let nor = [[0.0, 0.0, 1.0]; 4];
    let (c, n, h, aspect) = pane_geometry(&pos, &nor).unwrap();
    assert!(close(c, Vec3::new(0.2, 0.1, 0.0)) && close(n, Vec3::Z));
    assert!((h - 0.2).abs() < 1e-6, "extent along the up direction");
    assert!((aspect - 2.0).abs() < 1e-5, "{aspect}");
    // A portrait pane tilted back about X keeps its in-plane height.
    let tilt = Quat::from_rotation_x(0.3);
    let pos2: Vec<[f32; 3]> = [
        [0.0, 0.0, 0.0],
        [0.2, 0.0, 0.0],
        [0.2, 0.4, 0.0],
        [0.0, 0.4, 0.0],
    ]
    .iter()
    .map(|p| (tilt * Vec3::from(*p)).to_array())
    .collect();
    let nor2 = vec![(tilt * Vec3::Z).to_array(); 4];
    let (_, _, h2, a2) = pane_geometry(&pos2, &nor2).unwrap();
    assert!(
        (h2 - 0.4).abs() < 1e-4 && (a2 - 0.5).abs() < 1e-4,
        "{h2} {a2}"
    );
    assert!(pane_geometry(&[], &[]).is_none());
    assert!(pane_geometry(&pos, &[]).is_none());
    // A pane facing up uses the X axis as its up direction.
    let flat: Vec<[f32; 3]> = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [1.0, 0.0, 0.5],
        [0.0, 0.0, 0.5],
    ];
    let (_, _, hf, af) = pane_geometry(&flat, &[[0.0, 1.0, 0.0]; 4]).unwrap();
    assert!(
        (hf - 1.0).abs() < 1e-5 && (af - 0.5).abs() < 1e-5,
        "{hf} {af}"
    );
}

#[test]
fn the_uv_rectangle_of_the_pane_is_stretched_over_the_picture() {
    let (lo, hi) = uv_bounds(&[[0.16, -0.7], [0.89, -0.26], [0.5, -0.4]]).unwrap();
    assert!(close2(lo, Vec2::new(0.16, -0.7)) && close2(hi, Vec2::new(0.89, -0.26)));
    assert!(uv_bounds(&[]).is_none());
    let t = pane_uv_transform(lo, hi, false, false);
    assert!(close2(t.transform_point2(lo), Vec2::ZERO));
    assert!(close2(t.transform_point2(hi), Vec2::ONE));
    let f = pane_uv_transform(lo, hi, true, false);
    assert!(
        close2(f.transform_point2(lo), Vec2::new(1.0, 0.0)),
        "mirrored in u"
    );
    assert!(close2(f.transform_point2(hi), Vec2::new(0.0, 1.0)));
    let g = pane_uv_transform(lo, hi, false, true);
    assert!(
        close2(g.transform_point2(lo), Vec2::new(0.0, 1.0)),
        "turned in v"
    );
    // A degenerate rectangle does not blow up.
    let d = pane_uv_transform(Vec2::splat(0.5), Vec2::splat(0.5), false, false);
    assert!(d.transform_point2(Vec2::splat(0.5)).is_finite());
}
