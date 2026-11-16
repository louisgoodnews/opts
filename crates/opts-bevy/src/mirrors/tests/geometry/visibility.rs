//! Visibility of mirrors: in front of the camera, the turn pattern and the nearest pick.

use super::*;

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
