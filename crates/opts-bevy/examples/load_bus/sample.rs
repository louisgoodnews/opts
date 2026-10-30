//! Sample data of the `load_bus` example: the `.bus` text, its model config and demo meshes.

use bevy::prelude::*;
use opts_core::formats::vehicles::{
    bus::BusManifest,
    model::{
        InteriorLight, ModelConfig, ModelLod, ModelMaterial, ModelMesh, Spotlight, ViewpointFlags,
    },
};

/// Representative OMSI 2 `.bus` manifest.
pub(super) const BUS_TEXT: &str = r#"
[friendlyname]
MAN
NL202 - EN92
BVG Traditional Yellow

[description]
Standard 12m low-floor city transit bus with 2 doors.
Equipped with D0826 LOH07 Euro-2 engine and Voith DIWA automatic transmission.
[end]

[number]
Regs_EN92.org

[registration_automatic]
B-V

[sound]
sound\sound_NL202.cfg

[sound_ai]
sound\sound_NL202_ai.cfg

[model]
model\model_NL202.cfg

[paths]
paths\paths_NL202.cfg

[passengercabin]
passengercabin\passengercabin_NL202.cfg

[mass]
11.5

[newaxle]
3.2
2.45
1.95
0.95
85000
120000
9500
0

[newaxle]
-2.8
2.45
1.95
0.95
95000
140000
11000
1

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

[add_camera_driver]
0.35
3.92
1.18
-0.05
55
42
-28

[view_ticketselling]

[add_camera_pax]
-0.65
2.40
1.48
0.0
60
0
-2

[add_camera_pax]
0.0
-1.80
1.55
0.0
65
0
-1
"#;

pub(super) fn sample_manifest() -> BusManifest {
    BusManifest::parse(BUS_TEXT).expect("Valid sample .bus manifest")
}

/// Representative `ModelConfig` with multiple meshes and lights.
pub(super) fn sample_model() -> ModelConfig {
    let mut model = ModelConfig::new();
    // Meshes for LOD 0:
    // 0: Bus chassis body (yellow)
    // 1: Windshield & glass (tinted transparent)
    // 2: Dashboard cab interior (dark gray)
    // 3: Wheels (black rubber)
    let mut mesh_body = ModelMesh::new("body.o3d".to_string());
    mesh_body.materials.push(ModelMaterial {
        texture: "bvg_yellow.bmp".to_string(),
        slot_index: 0,
        ..Default::default()
    });

    let mut mesh_glass = ModelMesh::new("glass.o3d".to_string());
    mesh_glass.viewpoint = ViewpointFlags::ALWAYS;
    let glass_mat = ModelMaterial {
        texture: "window_tint.dds".to_string(),
        slot_index: 0,
        alpha_mode: Some(2), // Alpha blending
        ..Default::default()
    };
    mesh_glass.materials.push(glass_mat);

    let mut mesh_interior = ModelMesh::new("interior.o3d".to_string());
    mesh_interior.viewpoint = ViewpointFlags::INSIDE;

    let mut mesh_wheels = ModelMesh::new("wheels.o3d".to_string());
    mesh_wheels.viewpoint = ViewpointFlags::ALWAYS;

    model.lods.push(ModelLod {
        threshold: 0.0,
        meshes: vec![mesh_body, mesh_glass, mesh_interior, mesh_wheels],
    });

    // Interior lights: passenger cabin
    model.interior_lights.push(InteriorLight {
        variable: "cp_light_pax".to_string(),
        range: 6.5,
        color_rgb: [255, 245, 220],
        pos: Vec3::new(0.0, 0.5, 2.1),
    });

    // Exterior headlights: left and right
    model.spotlights.push(Spotlight {
        pos: Vec3::new(-0.85, 5.75, 0.72),
        dir: Vec3::new(0.0, 1.0, -0.05),
        color_rgb: [255, 255, 235],
        range: 45.0,
        inner_cone: 18.0,
        outer_cone: 35.0,
    });
    model.spotlights.push(Spotlight {
        pos: Vec3::new(0.85, 5.75, 0.72),
        dir: Vec3::new(0.0, 1.0, -0.05),
        color_rgb: [255, 255, 235],
        range: 45.0,
        inner_cone: 18.0,
        outer_cone: 35.0,
    });
    model
}

/// Demonstration meshes and materials for the bus hierarchy.
pub(super) fn demo_handles(
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) -> (Vec<Handle<Mesh>>, Vec<Handle<StandardMaterial>>) {
    let body_mesh_handle = meshes.add(Cuboid::new(2.5, 2.4, 11.5));
    let body_mat_handle = materials.add(StandardMaterial {
        base_color: Color::srgb(0.96, 0.77, 0.12), // BVG traffic yellow
        metallic: 0.2,
        perceptual_roughness: 0.35,
        ..default()
    });

    // Windshield glass
    let glass_mesh_handle = meshes.add(Plane3d::new(Vec3::NEG_Z, Vec2::new(1.15, 0.9)));
    let glass_mat_handle = materials.add(StandardMaterial {
        base_color: Color::srgba(0.8, 0.9, 1.0, 0.35),
        alpha_mode: AlphaMode::Blend,
        metallic: 0.1,
        perceptual_roughness: 0.1,
        ..default()
    });

    // Dashboard interior box
    let interior_mesh_handle = meshes.add(Cuboid::new(2.3, 1.8, 8.5));
    let interior_mat_handle = materials.add(StandardMaterial {
        base_color: Color::srgb(0.22, 0.22, 0.24),
        perceptual_roughness: 0.85,
        ..default()
    });

    // Wheels
    let wheel_mesh_handle = meshes.add(Cylinder::new(0.48, 0.32));
    let wheel_mat_handle = materials.add(StandardMaterial {
        base_color: Color::srgb(0.08, 0.08, 0.09),
        perceptual_roughness: 0.95,
        ..default()
    });

    let mesh_handles = vec![
        body_mesh_handle,
        glass_mesh_handle,
        interior_mesh_handle,
        wheel_mesh_handle,
    ];
    let mat_handles = vec![
        body_mat_handle,
        glass_mat_handle,
        interior_mat_handle,
        wheel_mat_handle,
    ];
    (mesh_handles, mat_handles)
}
