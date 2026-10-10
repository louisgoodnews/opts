//! Integration tests for scenery asset loading, spline extrusion, and object spawning in Bevy.

use bevy::app::App;
use bevy::asset::{AssetPlugin, Assets, Handle};
use bevy::core::Name;
use bevy::hierarchy::Children;
use bevy::math::Vec3;
use bevy::pbr::StandardMaterial;
use bevy::render::mesh::Mesh;
use bevy::transform::components::Transform;
use bevy::MinimalPlugins;

use opts_bevy::{
    camera_preset_to_bevy_transform, extrude_spline_profile, spawn_scenery_object, OmsiTransform,
    OptsSceneryPlugin, SceneryObjectAsset, SplineProfileAsset,
};
use opts_core::formats::scenery::{
    SceneryObject, ScoMesh, SplineGraphicalProfile, SplineProfile, SplineProfilePoint,
};

#[test]
fn test_opts_scenery_plugin_registration() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(AssetPlugin {
        watch_for_changes_override: Some(false),
        ..Default::default()
    });
    app.add_plugins(OptsSceneryPlugin);

    assert!(app.is_plugin_added::<OptsSceneryPlugin>());
    assert!(app
        .world()
        .get_resource::<Assets<SceneryObjectAsset>>()
        .is_some());
    assert!(app
        .world()
        .get_resource::<Assets<SplineProfileAsset>>()
        .is_some());
}

#[test]
fn test_camera_preset_to_bevy_transform() {
    // Driver camera: x = -0.70 (positive is left, so +0.70 in OMSI lateral),
    // y = 4.40 forward, z = 1.80 up, dist = -0.06, yaw = 0, pitch = -5.0 deg
    let cam_pos = Vec3::new(-0.70, 4.40, 1.80);
    let transform = camera_preset_to_bevy_transform(cam_pos, -0.06, 0.0, -5.0);

    // Bevy coordinates: x = 0.70, y ≈ 1.80, z ≈ -4.40
    assert!((transform.translation.x - 0.70).abs() < 1e-3);
    assert!((transform.translation.y - 1.80).abs() < 0.1);
    assert!((transform.translation.z - (-4.40)).abs() < 0.1);
}

#[test]
fn test_spline_profile_extrusion_geometry() {
    let mut profile = SplineProfile::default();
    profile.graphical_profiles.push(SplineGraphicalProfile {
        texture_index: 0,
        points: vec![
            SplineProfilePoint {
                x: -3.75,
                z: 0.10,
                u: 0.0,
                v_slope: 0.167,
            },
            SplineProfilePoint {
                x: 0.0,
                z: 0.10,
                u: 0.5,
                v_slope: 0.167,
            },
            SplineProfilePoint {
                x: 3.75,
                z: 0.10,
                u: 1.0,
                v_slope: 0.167,
            },
        ],
    });

    let length = 20.0;
    let meshes = extrude_spline_profile(&profile, length);
    assert_eq!(meshes.len(), 1);
    let (tex_idx, mesh) = &meshes[0];
    assert_eq!(*tex_idx, 0);

    // 3 points * 2 vertices each = 6 vertices
    let positions = mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap();
    assert_eq!(positions.len(), 6);

    // 2 segments * 2 triangles * 3 indices = 12 indices
    let indices = mesh.indices().unwrap();
    assert_eq!(indices.len(), 12);
}

#[test]
fn test_spawn_scenery_object_hierarchy() {
    let mut app = App::new();

    let mut sco = SceneryObject {
        friendly_name: Some("Test_Building".to_string()),
        ..Default::default()
    };
    sco.meshes.push(ScoMesh::new("building_base.o3d"));
    sco.meshes.push(ScoMesh::new("building_roof.o3d"));

    let spawn_transform = Transform::from_xyz(10.0, 0.0, -20.0);
    let mesh_handles: Vec<Handle<Mesh>> = vec![Handle::default(), Handle::default()];
    let mat_handles: Vec<Handle<StandardMaterial>> = vec![Handle::default(), Handle::default()];

    let parent_entity = {
        let mut commands = app.world_mut().commands();
        spawn_scenery_object(
            &mut commands,
            &sco,
            spawn_transform,
            &mesh_handles,
            &mat_handles,
        )
    };

    app.update();

    let world = app.world();
    assert!(world.get::<OmsiTransform>(parent_entity).is_some());
    assert_eq!(
        world.get::<Name>(parent_entity).map(|n| n.as_str()),
        Some("Test_Building")
    );

    let children = world
        .get::<Children>(parent_entity)
        .expect("must have children");
    assert_eq!(children.len(), 2);
}
