//! Shared test scene: a main camera, a player vehicle and the mirror panes.

use super::super::spawn::spawn_mirror_cameras;
use super::super::update::{sync_mirror_camera_look, update_mirror_cameras};
use super::*;
use crate::glass::register_surface;

pub(super) fn close2(a: Vec2, b: Vec2) -> bool {
    (a - b).length() < 1e-4
}

pub(super) struct Scene {
    pub(super) app: App,
    pub(super) main: Entity,
    pub(super) vehicle: Entity,
    /// Mirror mesh entities in the order given to `Scene::new`.
    pub(super) mirrors: Vec<Entity>,
    pub(super) originals: Vec<Handle<StandardMaterial>>,
}

impl Scene {
    /// A main camera at the origin looking down -Z (a wide view: 120 degrees high, aspect 2),
    /// a player vehicle with one mirror per given position (a quad facing +Z).
    pub(super) fn new(positions: &[Vec3], player: bool) -> Self {
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
            .add_systems(Update, (spawn_mirror_cameras, register_ambient_panes))
            .add_systems(
                PostUpdate,
                (
                    update_mirror_cameras,
                    update_ambient_mirrors,
                    sync_mirror_camera_look,
                )
                    .chain(),
            );
        let main = app
            .world_mut()
            .spawn((
                Camera3d::default(),
                MainCamera,
                Hdr,
                GlobalTransform::default(),
                Projection::Perspective(PerspectiveProjection {
                    fov: 120f32.to_radians(),
                    aspect_ratio: 2.0,
                    ..default()
                }),
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
    pub(super) fn add_pane(&mut self, position: Vec3, like: usize) -> Entity {
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
}

pub(super) fn three_mirrors() -> [Vec3; 3] {
    [
        Vec3::new(3.0, 0.0, -2.0),
        Vec3::new(0.0, 0.0, -2.0),
        Vec3::new(1.0, 0.0, -2.0),
    ]
}
