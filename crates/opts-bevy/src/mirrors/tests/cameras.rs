//! Mirror camera components (no main-camera match, HDR, fog and the 8-bit fallback).

use super::*;

#[test]
fn no_mirror_camera_matches_a_main_camera_query_and_all_carry_hdr() {
    let mut s = Scene::new(&three_mirrors(), true);
    s.quality(2, 1);
    s.app.update();
    s.app.update();
    let cams = s.cameras();
    assert_eq!(cams.len(), 3);
    let mut main_q = s
        .app
        .world_mut()
        .query_filtered::<Entity, (With<Camera3d>, With<MainCamera>)>();
    let mains: Vec<_> = main_q.iter(s.app.world()).collect();
    assert_eq!(mains, vec![s.main], "exactly the player's camera");
    for c in cams {
        assert!(s.app.world().get::<MainCamera>(c).is_none());
        assert!(
            s.app.world().get::<Hdr>(c).is_some(),
            "Hdr like the main camera"
        );
        assert!(s.app.world().get::<Camera3d>(c).is_some(), "never removed");
        assert_eq!(s.app.world().get::<Msaa>(c), Some(&Msaa::Off));
        assert_eq!(
            s.app.world().get::<Tonemapping>(c),
            Some(&Tonemapping::None)
        );
        assert!(matches!(
            s.app.world().get::<RenderTarget>(c),
            Some(RenderTarget::Image(_))
        ));
    }
}

#[test]
fn hdr_and_fog_follow_the_main_camera() {
    let mut s = Scene::new(&[Vec3::new(0.0, 0.0, -2.0)], true);
    s.quality(1, 1);
    s.app.update();
    let cam = s.cameras()[0];
    assert!(s.app.world().get::<DistanceFog>(cam).is_none());
    let fog = DistanceFog {
        color: Color::linear_rgb(0.4, 0.5, 0.6),
        ..default()
    };
    s.app.world_mut().entity_mut(s.main).insert(fog);
    s.app.update();
    s.app.update();
    assert_eq!(
        s.app.world().get::<DistanceFog>(cam).map(|f| f.color),
        Some(Color::linear_rgb(0.4, 0.5, 0.6))
    );
    s.app.world_mut().entity_mut(s.main).remove::<Hdr>();
    s.app.world_mut().entity_mut(s.main).remove::<DistanceFog>();
    s.app.update();
    s.app.update();
    assert!(s.app.world().get::<Hdr>(cam).is_none(), "8-bit main camera");
    assert!(s.app.world().get::<DistanceFog>(cam).is_none());
}
