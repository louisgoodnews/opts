//! The round-robin budget of the mirror cameras (`mirrors/schedule.rs`).

use super::*;

#[test]
fn renders_in_frame_spreads_the_budget_over_the_frames() {
    // Two pictures every 2nd frame: one per frame.
    assert_eq!((0..8).map(|f| renders_in_frame(f, 2, 2)).sum::<usize>(), 8);
    assert!((0..8).all(|f| renders_in_frame(f, 2, 2) == 1));
    // Two every frame, one every 4th frame, none.
    assert!((0..8).all(|f| renders_in_frame(f, 2, 1) == 2));
    assert_eq!((0..8).map(|f| renders_in_frame(f, 1, 4)).sum::<usize>(), 2);
    assert!((0..8).all(|f| renders_in_frame(f, 0, 1) == 0));
}

#[test]
fn pick_renders_takes_the_oldest_picture_then_the_nearest() {
    let last = [5, 0, 3, 3, 0];
    let distance = [1.0, 9.0, 2.0, 1.5, 4.0];
    let eligible = [true, true, true, true, false];
    assert_eq!(pick_renders(&last, &distance, &eligible, 1), vec![1]);
    assert_eq!(pick_renders(&last, &distance, &eligible, 3), vec![1, 3, 2]);
    assert_eq!(
        pick_renders(&last, &distance, &eligible, 9),
        vec![1, 3, 2, 0]
    );
    assert!(pick_renders(&last, &distance, &[false; 5], 2).is_empty());
}

#[test]
fn mirror_in_view_follows_the_view_pyramid() {
    let cam = GlobalTransform::default(); // at the origin, looking down -Z
    let fov = 40f32.to_radians();
    let inside = |p: Vec3, r: f32| mirror_in_view(&cam, fov, 1.8, p, r);
    assert!(inside(Vec3::new(0.0, 0.0, -3.0), 0.1));
    assert!(!inside(Vec3::new(0.0, 0.0, 3.0), 0.1), "behind");
    // Horizontal half-angle about 33 degrees at aspect 1.8: x = 1.8 at depth 3 is inside,
    // x = 3 is not, unless the pane is large enough to reach in.
    assert!(inside(Vec3::new(1.8, 0.0, -3.0), 0.05));
    assert!(!inside(Vec3::new(3.0, 0.0, -3.0), 0.05));
    assert!(inside(Vec3::new(3.0, 0.0, -3.0), 1.5));
    // Vertical half-angle 20 degrees: y = 1.5 at depth 3 is above the view.
    assert!(!inside(Vec3::new(0.0, 1.5, -3.0), 0.05));
    // A pane the camera stands in counts.
    assert!(inside(Vec3::new(0.0, 0.0, 0.05), 0.2));
    // The camera's own rotation counts: turned 90 degrees to the left, -X is ahead.
    let left = GlobalTransform::from(Transform::from_rotation(Quat::from_rotation_y(
        std::f32::consts::FRAC_PI_2,
    )));
    assert!(mirror_in_view(
        &left,
        fov,
        1.8,
        Vec3::new(-3.0, 0.0, 0.0),
        0.1
    ));
    assert!(!mirror_in_view(
        &left,
        fov,
        1.8,
        Vec3::new(0.0, 0.0, -3.0),
        0.1
    ));
}

#[test]
fn a_budget_of_one_serves_the_mirrors_in_turn() {
    let mut s = Scene::new(&three_mirrors(), true);
    s.quality(1, 1);
    s.app.update();
    assert!(s.shows_live(s.mirrors[1]) && s.active(s.camera_of(s.mirrors[1])));
    assert!(!s.shows_live(s.mirrors[2]) && !s.shows_live(s.mirrors[0]));
    let mut renders = [0usize; 3];
    for _ in 0..6 {
        s.app.update();
        let active: Vec<bool> = s
            .mirrors
            .clone()
            .iter()
            .map(|m| s.active(s.camera_of(*m)))
            .collect();
        assert_eq!(active.iter().filter(|a| **a).count(), 1, "{active:?}");
        for (i, a) in active.iter().enumerate() {
            renders[i] += usize::from(*a);
        }
    }
    assert_eq!(renders, [2, 2, 2], "round-robin");
}

#[test]
fn a_mirror_out_of_view_does_not_take_the_budget_from_one_in_view() {
    let mut s = Scene::new(
        &[Vec3::new(0.0, 0.0, -1.0), Vec3::new(0.0, 0.0, -6.0)],
        true,
    );
    s.app
        .world_mut()
        .entity_mut(s.main)
        .insert(Projection::Perspective(PerspectiveProjection {
            fov: 40f32.to_radians(),
            aspect_ratio: 1.8,
            ..default()
        }));
    // The near mirror sits off to the side, outside the 40 degree view.
    let mut t = s
        .app
        .world_mut()
        .get_mut::<GlobalTransform>(s.mirrors[0])
        .unwrap();
    *t = GlobalTransform::from_translation(Vec3::new(3.0, 0.0, -1.0));
    s.quality(1, 1);
    for _ in 0..3 {
        s.app.update();
        assert!(!s.active(s.camera_of(s.mirrors[0])), "beside the view");
        assert!(s.active(s.camera_of(s.mirrors[1])), "in view, farther");
    }
    assert_eq!(s.app.world().resource::<MirrorStats>().selected, 1);
}
