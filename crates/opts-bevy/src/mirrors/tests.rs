//! Unit tests of `mirrors`.

use super::spawn::spawn_mirror_cameras;
use super::update::{sync_mirror_camera_look, update_mirror_cameras};
use super::*;
use crate::glass::register_surface;

// ----- the maths ------------------------------------------------------------------------------

mod geometry;
use geometry::close;

fn close2(a: Vec2, b: Vec2) -> bool {
    (a - b).length() < 1e-4
}

// ----- the systems ----------------------------------------------------------------------------

struct Scene {
    app: App,
    main: Entity,
    vehicle: Entity,
    /// Mirror mesh entities in the order given to `Scene::new`.
    mirrors: Vec<Entity>,
    originals: Vec<Handle<StandardMaterial>>,
}

impl Scene {
    /// A main camera at the origin looking down -Z, a player vehicle with one mirror per given
    /// position (a quad facing +Z).
    fn new(positions: &[Vec3], player: bool) -> Self {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<Assets<Image>>()
            .init_resource::<Assets<StandardMaterial>>()
            .init_resource::<Assets<Mesh>>()
            .insert_resource(LightQuality::default())
            .init_resource::<MirrorSettings>()
            .init_resource::<MirrorFrame>()
            .init_resource::<MirrorStats>()
            .init_resource::<VirtualMirrors>()
            .add_systems(Update, spawn_mirror_cameras)
            .add_systems(PostUpdate, (update_mirror_cameras, sync_mirror_camera_look));
        let main = app
            .world_mut()
            .spawn((
                Camera3d::default(),
                MainCamera,
                Hdr,
                GlobalTransform::default(),
            ))
            .id();
        let mesh = app
            .world_mut()
            .resource_mut::<Assets<Mesh>>()
            .add(Rectangle::new(0.4, 0.2));
        let vehicle = app.world_mut().spawn(Transform::default()).id();
        if player {
            app.world_mut().entity_mut(vehicle).insert(PlayerVehicle);
        }
        let mut mirrors = Vec::new();
        let mut originals = Vec::new();
        for p in positions {
            let mat = StandardMaterial {
                base_color: Color::srgb(0.5, 0.5, 0.6),
                ..default()
            };
            let handle = crate::glass::unique_material(
                &mut app.world_mut().resource_mut::<Assets<StandardMaterial>>(),
                mat.clone(),
            );
            register_surface(&handle, SurfaceKind::Mirror, &mat);
            let e = app
                .world_mut()
                .spawn((
                    Mesh3d(mesh.clone()),
                    MeshMaterial3d(handle.clone()),
                    Transform::from_translation(*p),
                    GlobalTransform::from_translation(*p),
                ))
                .id();
            app.world_mut().entity_mut(vehicle).add_child(e);
            mirrors.push(e);
            originals.push(handle);
        }
        Self {
            app,
            main,
            vehicle,
            mirrors,
            originals,
        }
    }

    /// Another pane at `position` that shares the material of pane `like` (a second slot or a
    /// variant of the same mirror).
    fn add_pane(&mut self, position: Vec3, like: usize) -> Entity {
        let mesh = self
            .app
            .world()
            .get::<Mesh3d>(self.mirrors[like])
            .unwrap()
            .0
            .clone();
        let e = self
            .app
            .world_mut()
            .spawn((
                Mesh3d(mesh),
                MeshMaterial3d(self.originals[like].clone()),
                Transform::from_translation(position),
                GlobalTransform::from_translation(position),
            ))
            .id();
        self.app.world_mut().entity_mut(self.vehicle).add_child(e);
        self.mirrors.push(e);
        self.originals.push(self.originals[like].clone());
        e
    }

    fn quality(&mut self, count: u8, every: u8) {
        let mut q = self.app.world_mut().resource_mut::<LightQuality>();
        q.mirror_count = count;
        q.mirror_every = every;
    }

    fn cameras(&mut self) -> Vec<Entity> {
        let mut q = self
            .app
            .world_mut()
            .query_filtered::<Entity, With<MirrorCamera>>();
        q.iter(self.app.world()).collect()
    }

    fn active(&self, camera: Entity) -> bool {
        self.app.world().get::<Camera>(camera).unwrap().is_active
    }

    fn camera_of(&self, mirror: Entity) -> Entity {
        self.app
            .world()
            .get::<MirrorSurface>(mirror)
            .unwrap()
            .camera
    }

    fn shows_live(&self, mirror: Entity) -> bool {
        let camera = self.camera_of(mirror);
        let info = self.app.world().get::<MirrorCamera>(camera).unwrap();
        self.app
            .world()
            .get::<MeshMaterial3d<StandardMaterial>>(mirror)
            .unwrap()
            .0
            == info.dynamic
    }
}

fn three_mirrors() -> [Vec3; 3] {
    [
        Vec3::new(3.0, 0.0, -2.0),
        Vec3::new(0.0, 0.0, -2.0),
        Vec3::new(1.0, 0.0, -2.0),
    ]
}

#[test]
fn mirror_count_two_selects_the_nearest_two_and_the_rest_keep_their_texture() {
    let mut s = Scene::new(&three_mirrors(), true);
    s.quality(2, 1);
    s.app.update();
    assert_eq!(s.cameras().len(), 3, "one camera per mirror mesh");
    // Distances: mirror 1 (2.0), mirror 2 (2.24), mirror 0 (3.6).
    assert!(s.shows_live(s.mirrors[1]) && s.shows_live(s.mirrors[2]));
    assert!(
        !s.shows_live(s.mirrors[0]),
        "the third keeps the static texture"
    );
    assert!(s.active(s.camera_of(s.mirrors[1])) && s.active(s.camera_of(s.mirrors[2])));
    assert!(!s.active(s.camera_of(s.mirrors[0])));
    let st = *s.app.world().resource::<MirrorStats>();
    assert_eq!((st.cameras, st.selected, st.rendering), (3, 2, 2));
}

#[test]
fn mirror_count_one_selects_only_the_nearest() {
    let mut s = Scene::new(&three_mirrors(), true);
    s.quality(1, 1);
    s.app.update();
    assert!(s.shows_live(s.mirrors[1]));
    assert!(!s.shows_live(s.mirrors[2]) && !s.shows_live(s.mirrors[0]));
    assert!(s.active(s.camera_of(s.mirrors[1])));
    assert!(!s.active(s.camera_of(s.mirrors[2])));
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
            for (rank, m) in s.mirrors.clone().iter().enumerate() {
                let active = s.active(s.camera_of(*m));
                assert_eq!(
                    active,
                    is_render_frame(frame, rank, every),
                    "every {every}, frame {frame}, rank {rank}"
                );
                seen[rank].push(active);
            }
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
    assert_eq!(
        s.app
            .world()
            .get::<MeshMaterial3d<StandardMaterial>>(s.mirrors[0])
            .unwrap()
            .0,
        s.originals[0]
    );
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

mod tags;
mod virtual_view;
