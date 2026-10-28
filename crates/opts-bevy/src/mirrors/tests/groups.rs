//! Tests for mirror camera caps, pose, shared-material pane groups, visibility and UV mapping.

use super::*;

#[test]
fn vehicles_that_are_not_the_players_get_no_mirror_cameras() {
    let mut s = Scene::new(&three_mirrors(), false);
    s.quality(2, 1);
    s.app.update();
    s.app.update();
    assert!(s.cameras().is_empty());
    for m in &s.mirrors {
        assert!(s.app.world().get::<MirrorSurface>(*m).is_none());
    }
}

#[test]
fn mirrors_are_capped_and_a_removed_vehicle_takes_its_cameras_along() {
    let many: Vec<Vec3> = (0..MAX_CAMERAS + 2)
        .map(|i| Vec3::new(i as f32 * 0.5, 0.0, -2.0))
        .collect();
    let mut s = Scene::new(&many, true);
    s.quality(2, 1);
    s.app.update();
    assert_eq!(s.cameras().len(), MAX_CAMERAS);
    s.app
        .world_mut()
        .entity_mut(s.vehicle)
        .remove::<PlayerVehicle>();
    s.app.update();
    s.app.update();
    assert!(s.cameras().is_empty(), "no longer the player's vehicle");
    // Its panes are no longer the player's: the camera system has let go of them (the nearest
    // may serve as another vehicle's mirror, see `ambient`).
    assert!(s.app.world().get::<MirrorSurface>(s.mirrors[0]).is_none());
}

#[test]
fn the_camera_is_posed_at_the_mirror_looking_back_at_the_reflected_ray() {
    let mut s = Scene::new(&[Vec3::new(0.0, 0.0, -4.0)], true);
    s.quality(1, 1);
    s.app.update();
    let cam = s.cameras()[0];
    let t = *s.app.world().get::<Transform>(cam).unwrap();
    assert!(
        close(t.translation, Vec3::new(0.0, 0.0, -4.0 + MIRROR_OFFSET_M)),
        "{t:?}"
    );
    assert!(close(*t.forward(), Vec3::Z), "{:?}", t.forward());
    let Projection::Perspective(p) = s.app.world().get::<Projection>(cam).unwrap() else {
        panic!("perspective");
    };
    assert!((p.fov - mirror_fov(0.2, 4.0)).abs() < 1e-3);
}

#[test]
fn panes_that_share_a_material_share_one_camera_and_the_visible_one_sets_the_pose() {
    let mut s = Scene::new(&[Vec3::new(0.0, 0.0, -4.0)], true);
    // A variant of the same mirror, hidden by the model's visibility rules, and far off to the side.
    let hidden = s.add_pane(Vec3::new(5.0, 0.0, -4.0), 0);
    s.app
        .world_mut()
        .entity_mut(hidden)
        .insert(InheritedVisibility::HIDDEN);
    s.quality(2, 1);
    s.app.update();
    assert_eq!(s.cameras().len(), 1, "one camera for the shared material");
    assert_eq!(s.camera_of(s.mirrors[0]), s.camera_of(hidden));
    let cam = s.cameras()[0];
    let info = s.app.world().get::<MirrorCamera>(cam).unwrap();
    assert_eq!(info.members.len(), 2);
    let t = *s.app.world().get::<Transform>(cam).unwrap();
    assert!(
        close(t.translation, Vec3::new(0.0, 0.0, -4.0 + MIRROR_OFFSET_M)),
        "the hidden variant does not move the camera: {t:?}"
    );
    assert!(
        s.shows_live(hidden) && s.shows_live(s.mirrors[0]),
        "both show the picture"
    );
    // A pane that appears later joins the group at the next scan.
    s.app
        .world_mut()
        .resource_mut::<MirrorSettings>()
        .scan_interval_s = 0.0;
    let late = s.add_pane(Vec3::new(0.2, 0.0, -4.0), 0);
    s.app.update();
    s.app.update();
    assert_eq!(s.cameras().len(), 1, "still one camera");
    assert_eq!(s.camera_of(late), cam);
    assert_eq!(
        s.app
            .world()
            .get::<MirrorCamera>(cam)
            .unwrap()
            .members
            .len(),
        3
    );
    assert!(s.shows_live(late));
}

#[test]
fn a_group_without_a_visible_pane_is_never_selected() {
    let mut s = Scene::new(
        &[Vec3::new(0.0, 0.0, -2.0), Vec3::new(1.0, 0.0, -2.0)],
        true,
    );
    s.app
        .world_mut()
        .entity_mut(s.mirrors[0])
        .insert(InheritedVisibility::HIDDEN);
    s.quality(1, 1);
    s.app.update();
    assert!(
        !s.shows_live(s.mirrors[0]),
        "hidden group keeps its texture"
    );
    assert!(s.shows_live(s.mirrors[1]), "the visible one takes the slot");
    assert!(s.active(s.camera_of(s.mirrors[1])));
}

#[test]
fn the_picture_is_mapped_over_the_pane_uv_rectangle_and_the_camera_has_the_pane_aspect() {
    let mut s = Scene::new(&[Vec3::new(0.0, 0.0, -2.0)], true);
    s.quality(1, 1);
    s.app.update();
    let cam = s.cameras()[0];
    let info = s.app.world().get::<MirrorCamera>(cam).unwrap().clone();
    // The Rectangle mesh has UVs 0..1.
    assert!(close2(info.uv_min, Vec2::ZERO) && close2(info.uv_max, Vec2::ONE));
    let mats = s.app.world().resource::<Assets<StandardMaterial>>();
    let m = mats.get(&info.dynamic).unwrap();
    assert!(m.unlit && m.base_color_texture.is_some());
    assert!(close2(
        m.uv_transform.transform_point2(Vec2::ONE),
        Vec2::ONE
    ));
    let Projection::Perspective(p) = s.app.world().get::<Projection>(cam).unwrap() else {
        panic!("perspective");
    };
    assert!(
        (p.aspect_ratio - 2.0).abs() < 1e-3,
        "0.4 x 0.2 pane: {}",
        p.aspect_ratio
    );
}
