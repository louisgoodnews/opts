use bevy::prelude::*;
use opts_bevy::{spawn_vehicle_model, BusVehicle, OptsCoordsPlugin};
use opts_core::formats::vehicles::{
    bus::BusManifest,
    model::{InteriorLight, ModelConfig, ModelLod, ModelMesh, Spotlight, ViewpointFlags},
};

#[test]
fn test_spawn_vehicle_model_hierarchy() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(AssetPlugin {
        watch_for_changes_override: Some(false),
        ..Default::default()
    });
    app.init_resource::<Assets<Mesh>>();
    app.init_resource::<Assets<StandardMaterial>>();
    app.add_plugins(OptsCoordsPlugin);

    let bus_text = r#"
[friendlyname]
MAN
NL202
BVG Traditional

[mass]
11.5

[set_camera_std]
0

[set_camera_outside_center]
0.0
0.5
1.4

[add_camera_driver]
0.68
4.15
1.42
-0.08
52
0
-4

[add_camera_driver]
0.42
3.85
1.25
-0.05
55
-38
-22

[view_schedule]

[add_camera_pax]
-0.65
2.40
1.48
0.0
60
0
-2
"#;

    let manifest = BusManifest::parse(bus_text).expect("Valid bus text");

    let mut model = ModelConfig::new();
    let mesh_body = ModelMesh::new("body.o3d".to_string());
    let mut mesh_glass = ModelMesh::new("glass.o3d".to_string());
    mesh_glass.viewpoint = ViewpointFlags::ALWAYS;

    model.lods.push(ModelLod {
        threshold: 0.0,
        meshes: vec![mesh_body, mesh_glass],
    });

    model.interior_lights.push(InteriorLight {
        variable: "light_pax".to_string(),
        range: 6.0,
        color_rgb: [255, 240, 200],
        pos: Vec3::new(0.0, 1.0, 2.0),
    });

    model.spotlights.push(Spotlight {
        pos: Vec3::new(-0.8, 5.5, 0.7),
        dir: Vec3::new(0.0, 1.0, 0.0),
        color_rgb: [255, 255, 255],
        range: 40.0,
        inner_cone: 20.0,
        outer_cone: 30.0,
    });

    let mut meshes = app.world_mut().resource_mut::<Assets<Mesh>>();
    let body_handle = meshes.add(Cuboid::new(2.5, 2.5, 12.0));
    let glass_handle = meshes.add(Plane3d::new(Vec3::NEG_Z, Vec2::new(1.0, 1.0)));

    let mut materials = app.world_mut().resource_mut::<Assets<StandardMaterial>>();
    let body_mat = materials.add(StandardMaterial::default());
    let glass_mat = materials.add(StandardMaterial::default());

    let mesh_handles = vec![body_handle, glass_handle];
    let mat_handles = vec![body_mat, glass_mat];

    let spawn_transform = Transform::from_xyz(0.0, 1.5, 0.0);

    let mut commands = app.world_mut().commands();
    let vehicle_entity = spawn_vehicle_model(
        &mut commands,
        &manifest,
        &model,
        spawn_transform,
        &mesh_handles,
        &mat_handles,
    );

    app.update();

    // Verify parent vehicle entity components
    let vehicle = app
        .world()
        .get::<BusVehicle>(vehicle_entity)
        .expect("BusVehicle component present");
    assert_eq!(vehicle.friendly_name, "MAN NL202");
    assert_eq!(vehicle.mass_tonnes, Some(11.5));
    assert_eq!(vehicle.driver_cameras.len(), 2);
    assert_eq!(vehicle.pax_cameras.len(), 1);
    assert_eq!(vehicle.active_camera_index, 0);

    // Verify child attachments
    let children = app
        .world()
        .get::<Children>(vehicle_entity)
        .expect("Vehicle has children");
    // 2 meshes + 1 interior light + 1 spotlight = 4 children
    assert_eq!(children.len(), 4);

    // Verify camera transform method
    let cam_tx = vehicle
        .current_camera_bevy_transform()
        .expect("Camera transform exists");
    // OMSI driver camera: X=0.68 (left in cam frame -> -0.68 in OMSI -> -0.68 in Bevy X)
    assert!((cam_tx.translation.x - (-0.68)).abs() < 1e-3);
}
