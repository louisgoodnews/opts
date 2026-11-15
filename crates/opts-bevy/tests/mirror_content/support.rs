//! The test app of the mirror content tests: the loader, the parts and a player vehicle.

use std::path::Path;


pub(super) fn build_mirror_test_app() -> bevy::prelude::App {
    use bevy::prelude::*;
    use opts_bevy::mirrors::{
        spawn_mirror_cameras, update_mirror_cameras, MirrorFrame, MirrorSettings, MirrorStats,
    };

    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .init_resource::<Assets<Image>>()
        .init_resource::<Assets<StandardMaterial>>()
        .init_resource::<Assets<Mesh>>()
        .insert_resource(opts_bevy::light_quality::LightQuality::default())
        .insert_resource(MirrorSettings {
            scan_interval_s: 0.0,
            ..default()
        })
        .init_resource::<MirrorFrame>()
        .init_resource::<MirrorStats>()
        .init_resource::<opts_bevy::mirrors::VirtualMirrors>()
        .add_systems(
            Update,
            (
                spawn_mirror_cameras,
                opts_bevy::mirrors::register_ambient_panes,
            ),
        )
        .add_systems(
            PostUpdate,
            (
                update_mirror_cameras,
                opts_bevy::mirrors::update_ambient_mirrors,
            )
                .chain(),
        );
    app.world_mut().spawn((
        Camera3d::default(),
        opts_bevy::MainCamera,
        GlobalTransform::default(),
    ));
    app
}

pub(super) fn load_mirror_parts(
    app: &mut bevy::prelude::App,
    install: &opts_bevy::OmsiInstall,
    dir: &Path,
) -> Vec<opts_bevy::model::ModelPart> {
    use bevy::prelude::*;
    use opts_bevy::{ContentCache, ContentLoader};

    let mut cache = ContentCache::default();
    let mut parts = Vec::new();
    let world = app.world_mut();
    let mut meshes = world.remove_resource::<Assets<Mesh>>().unwrap();
    let mut materials = world.remove_resource::<Assets<StandardMaterial>>().unwrap();
    let mut images = world.remove_resource::<Assets<Image>>().unwrap();
    for file in [
        "Spiegel_links.o3d",
        "Spiegel_rechts_II.o3d",
        "Innenspiegel.o3d",
    ] {
        let mut loader = ContentLoader {
            install,
            cache: &mut cache,
            meshes: &mut meshes,
            materials: &mut materials,
            images: &mut images,
        };
        parts.extend(loader.o3d_parts(&dir.join(file), &[], &[]));
    }
    world.insert_resource(meshes);
    world.insert_resource(materials);
    world.insert_resource(images);
    parts
}

pub(super) fn spawn_test_vehicle(
    app: &mut bevy::prelude::App,
    parts: &[opts_bevy::model::ModelPart],
    player: bool,
) {
    use bevy::prelude::*;
    use opts_bevy::PlayerVehicle;

    let root = app.world_mut().spawn(Transform::default()).id();
    if player {
        app.world_mut().entity_mut(root).insert(PlayerVehicle);
    }
    let mut cmds = Vec::new();
    for (i, part) in parts.iter().enumerate() {
        cmds.push(
            app.world_mut()
                .spawn((
                    Mesh3d(part.mesh.clone()),
                    MeshMaterial3d(part.material.clone()),
                    Transform::from_xyz(i as f32 * 0.5, 0.0, -3.0),
                    GlobalTransform::from_xyz(i as f32 * 0.5, 0.0, -3.0),
                ))
                .id(),
        );
    }
    for c in cmds {
        app.world_mut().entity_mut(root).add_child(c);
    }
}

