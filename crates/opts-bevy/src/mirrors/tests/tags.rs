//! Tag-driven mirror cameras on buses.

use super::geometry::close;
use super::*;

fn tag(x: f32, y: f32, z: f32, fov: f32, yaw: f32, pitch: f32) -> BusMirrorCamera {
    BusMirrorCamera {
        x,
        y,
        z,
        param4: 0.0,
        fov_deg: fov,
        yaw_deg: yaw,
        pitch_deg: pitch,
        param8: None,
    }
}

#[test]
fn a_tag_camera_at_180_degrees_looks_back() {
    let (pos, rot, fov) = tag_pose(
        &tag(1.0, 5.0, 2.0, 52.0, 180.0, 0.0),
        &MirrorAdjust::default(),
    );
    assert!(close(pos, Vec3::new(1.0, 2.0, -5.0)), "{pos:?}");
    assert!(close(rot * Vec3::NEG_Z, Vec3::Z), "{:?}", rot * Vec3::NEG_Z);
    assert!((fov - 52f32.to_radians()).abs() < 1e-5);
}

#[test]
fn positive_yaw_turns_counter_clockwise_and_negative_pitch_looks_down() {
    let (_, rot, _) = tag_pose(
        &tag(0.0, 0.0, 0.0, 50.0, 90.0, -10.0),
        &MirrorAdjust::default(),
    );
    let d = rot * Vec3::NEG_Z;
    assert!(d.x < -0.9, "90 degrees is to the left: {d:?}");
    assert!(d.y < 0.0);
}

#[test]
fn the_user_adjustment_is_added_and_the_fov_stays_sane() {
    let adj = MirrorAdjust {
        yaw_deg: 10.0,
        pitch_deg: 5.0,
        fov_deg: 200.0,
    };
    let (_, rot, fov) = tag_pose(&tag(0.0, 0.0, 0.0, 50.0, 170.0, -10.0), &adj);
    // 170 + 10 = 180: straight back.
    let (_, plain, _) = tag_pose(
        &tag(0.0, 0.0, 0.0, 50.0, 180.0, -5.0),
        &MirrorAdjust::default(),
    );
    assert!(rot.angle_between(plain) < 1e-4);
    assert!(fov <= 160f32.to_radians() + 1e-5);
}

#[test]
fn the_pane_index_is_read_from_the_texture_name() {
    use crate::glass::mirror_index_of;
    assert_eq!(mirror_index_of("Texture\\Reflexion3.bmp"), Some(3));
    assert_eq!(mirror_index_of("reflexion12.bmp"), Some(12));
    assert_eq!(mirror_index_of("aussenspiegel.bmp"), None);
}

fn bus_with_tags(tags: Vec<BusMirrorCamera>) -> BusVehicle {
    BusVehicle {
        friendly_name: "T".into(),
        mass_tonnes: None,
        driver_cameras: Vec::new(),
        pax_cameras: Vec::new(),
        mirror_cameras: tags,
        trailer_pax_cameras: Vec::new(),
        trailer_driver_cameras: Vec::new(),
        outside_camera_center: None,
        std_camera_index: 0,
        active_camera_index: 0,
    }
}

#[test]
fn a_pane_with_a_bus_tag_gets_a_square_tag_driven_camera() {
    let mut s = Scene::new(&[Vec3::new(0.0, 0.0, -2.0)], true);
    let id = s.originals[0].id();
    crate::glass::set_mirror_index(id, 0);
    s.app
        .world_mut()
        .entity_mut(s.vehicle)
        .insert(bus_with_tags(vec![tag(-1.0, 5.0, 2.0, 52.0, 170.0, -10.0)]));
    s.quality(1, 1);
    s.app.update();
    let cam = s.cameras()[0];
    let info = s.app.world().get::<MirrorCamera>(cam).unwrap().clone();
    assert!(info.tag.is_some());
    let w = s.app.world();
    let image = w
        .resource::<Assets<Image>>()
        .get(&match w.get::<RenderTarget>(cam).unwrap() {
            RenderTarget::Image(i) => i.handle.clone(),
            _ => panic!("image target"),
        })
        .unwrap();
    assert_eq!(image.width(), image.height(), "square, like reflexionN.bmp");
    let Projection::Perspective(p) = w.get::<Projection>(cam).unwrap() else {
        panic!("perspective");
    };
    assert!((p.aspect_ratio - 1.0).abs() < 1e-5);
    assert!((p.fov - 52f32.to_radians()).abs() < 1e-4);
    let t = *w.get::<Transform>(cam).unwrap();
    assert!(close(t.translation, Vec3::new(-1.0, 2.0, -5.0)), "{t:?}");
    // The UVs are laid on the texture as they are (no stretching of the pane's rectangle).
    let m = w
        .resource::<Assets<StandardMaterial>>()
        .get(&info.dynamic)
        .unwrap();
    assert!(close2(
        m.uv_transform.transform_point2(Vec2::new(0.25, 0.75)),
        Vec2::new(0.25, 0.75)
    ));
}

#[test]
fn without_a_tag_for_the_pane_or_with_tags_off_the_reflection_is_used() {
    let mut s = Scene::new(&[Vec3::new(0.0, 0.0, -2.0)], true);
    crate::glass::set_mirror_index(s.originals[0].id(), 3);
    s.app
        .world_mut()
        .entity_mut(s.vehicle)
        .insert(bus_with_tags(vec![tag(-1.0, 5.0, 2.0, 52.0, 170.0, -10.0)]));
    s.quality(1, 1);
    s.app.update();
    let cam = s.cameras()[0];
    assert!(s
        .app
        .world()
        .get::<MirrorCamera>(cam)
        .unwrap()
        .tag
        .is_none());

    let mut off = Scene::new(&[Vec3::new(0.0, 0.0, -2.0)], true);
    crate::glass::set_mirror_index(off.originals[0].id(), 0);
    off.app
        .world_mut()
        .entity_mut(off.vehicle)
        .insert(bus_with_tags(vec![tag(-1.0, 5.0, 2.0, 52.0, 170.0, -10.0)]));
    off.app
        .world_mut()
        .resource_mut::<MirrorSettings>()
        .use_tags = false;
    off.quality(1, 1);
    off.app.update();
    let cam = off.cameras()[0];
    assert!(off
        .app
        .world()
        .get::<MirrorCamera>(cam)
        .unwrap()
        .tag
        .is_none());
}
