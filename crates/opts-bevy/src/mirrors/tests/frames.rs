//! Live pictures of the mirrors: the budget, the turn-taking and the view test.

use super::*;

#[test]
fn every_mirror_in_view_goes_live_and_the_budget_takes_turns() {
    let mut s = Scene::new(&three_mirrors(), true);
    s.quality(2, 1);
    s.app.update();
    assert_eq!(s.cameras().len(), 3, "one camera per mirror mesh");
    // Distances: mirror 1 (2.0), mirror 2 (2.24), mirror 0 (3.6). The two nearest go first.
    assert!(s.shows_live(s.mirrors[1]) && s.shows_live(s.mirrors[2]));
    assert!(
        !s.shows_live(s.mirrors[0]),
        "not rendered yet: the static texture"
    );
    assert!(s.active(s.camera_of(s.mirrors[1])) && s.active(s.camera_of(s.mirrors[2])));
    assert!(!s.active(s.camera_of(s.mirrors[0])));
    let st = *s.app.world().resource::<MirrorStats>();
    assert_eq!((st.cameras, st.selected, st.rendering), (3, 3, 2));
    // Next frame: the never-rendered mirror first, then the oldest-nearest one.
    s.app.update();
    assert!(s.active(s.camera_of(s.mirrors[0])) && s.active(s.camera_of(s.mirrors[1])));
    assert!(!s.active(s.camera_of(s.mirrors[2])));
    for m in s.mirrors.clone() {
        assert!(s.shows_live(m), "every mirror in view keeps a live picture");
    }
}

#[test]
fn mirror_count_zero_makes_no_cameras_and_removes_existing_ones() {
    let mut s = Scene::new(&three_mirrors(), true);
    s.quality(0, 2);
    s.app.update();
    s.app.update();
    assert!(s.cameras().is_empty());
    assert!(s.app.world().get::<MirrorSurface>(s.mirrors[0]).is_none());

    s.quality(2, 1);
    s.app.update();
    assert_eq!(s.cameras().len(), 3);
    s.quality(0, 1);
    s.app.update();
    s.app.update();
    assert!(s.cameras().is_empty(), "cameras despawned");
    for (m, original) in s.mirrors.clone().iter().zip(s.originals.clone()) {
        assert!(s.app.world().get::<MirrorSurface>(*m).is_none());
        assert_eq!(
            s.app
                .world()
                .get::<MeshMaterial3d<StandardMaterial>>(*m)
                .unwrap()
                .0,
            original,
            "the static texture is back"
        );
    }
}

#[test]
fn is_active_toggles_on_the_expected_frames() {
    for every in [1u8, 2, 4] {
        let mut s = Scene::new(
            &[Vec3::new(0.0, 0.0, -2.0), Vec3::new(1.0, 0.0, -2.0)],
            true,
        );
        s.quality(2, every);
        let mut seen = [Vec::new(), Vec::new()];
        s.app.update();
        for _ in 0..8 {
            let frame = s.app.world().resource::<MirrorFrame>().0;
            let mut active = 0;
            for (rank, m) in s.mirrors.clone().iter().enumerate() {
                let a = s.active(s.camera_of(*m));
                active += usize::from(a);
                seen[rank].push(a);
            }
            assert_eq!(
                active,
                renders_in_frame(frame, 2, every),
                "every {every}, frame {frame}"
            );
            s.app.update();
        }
        for pattern in &seen {
            assert_eq!(
                pattern.iter().filter(|a| **a).count(),
                8 / usize::from(every),
                "every {every}: {pattern:?}"
            );
        }
    }
}

#[test]
fn a_mirror_behind_the_main_camera_does_not_render() {
    let mut s = Scene::new(&[Vec3::new(0.0, 0.0, 3.0), Vec3::new(0.0, 0.0, -3.0)], true);
    s.quality(2, 1);
    s.app.update();
    assert!(!s.active(s.camera_of(s.mirrors[0])), "behind");
    assert!(s.active(s.camera_of(s.mirrors[1])), "in front");
}
