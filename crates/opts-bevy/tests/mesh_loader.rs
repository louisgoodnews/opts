use bevy::app::App;
use bevy::asset::AssetPlugin;
use bevy::math::{Vec2, Vec3};
use bevy::mesh::{Indices, Mesh, VertexAttributeValues};
use bevy::MinimalPlugins;

use opts_bevy::{
    o3d_to_bevy_mesh, x_mesh_to_bevy_meshes, x_mesh_to_combined_bevy_mesh, x_submesh_to_bevy_mesh,
    OptsMeshPlugin,
};
use opts_core::formats::o3d::{O3d, Vertex};
use opts_core::formats::x::{XFrame, XMaterial, XMesh, XSubMesh, XVertex};

#[test]
fn test_o3d_coordinate_conversion_and_winding() {
    let vertices = vec![
        Vertex {
            position: Vec3::new(1.0, 2.0, 3.0),
            normal: Vec3::new(0.0, 1.0, 0.5),
            uv: Vec2::new(0.2, 0.8),
            color: None,
        },
        Vertex {
            position: Vec3::new(-1.0, 0.0, 5.0),
            normal: Vec3::new(1.0, 0.0, -0.5),
            uv: Vec2::new(0.4, 0.6),
            color: None,
        },
        Vertex {
            position: Vec3::new(0.0, 3.0, -2.0),
            normal: Vec3::new(0.0, 0.0, 1.0),
            uv: Vec2::new(0.9, 0.1),
            color: None,
        },
    ];

    let indices = vec![[0, 1, 2]];

    let o3d = O3d {
        version: 1,
        header_flags: 0,
        vertices,
        indices,
        face_materials: vec![0],
        materials: Vec::new(),
        transform: None,
        unknown_chunks: Vec::new(),
        encryption_key: None,
        descrambled: false,
        exact_decoded: false,
    };

    let mesh = o3d_to_bevy_mesh(&o3d);

    // 1. Verify positions: Z negated
    if let Some(VertexAttributeValues::Float32x3(positions)) =
        mesh.attribute(Mesh::ATTRIBUTE_POSITION)
    {
        assert_eq!(positions[0], [1.0, 2.0, -3.0]);
        assert_eq!(positions[1], [-1.0, 0.0, -5.0]);
        assert_eq!(positions[2], [0.0, 3.0, 2.0]);
    } else {
        panic!("Missing ATTRIBUTE_POSITION");
    }

    // 2. Verify normals: Z negated
    if let Some(VertexAttributeValues::Float32x3(normals)) = mesh.attribute(Mesh::ATTRIBUTE_NORMAL)
    {
        assert_eq!(normals[0], [0.0, 1.0, -0.5]);
        assert_eq!(normals[1], [1.0, 0.0, 0.5]);
        assert_eq!(normals[2], [0.0, 0.0, -1.0]);
    } else {
        panic!("Missing ATTRIBUTE_NORMAL");
    }

    // 3. Verify UVs
    if let Some(VertexAttributeValues::Float32x2(uvs)) = mesh.attribute(Mesh::ATTRIBUTE_UV_0) {
        assert_eq!(uvs[0], [0.2, 0.8]);
        assert_eq!(uvs[1], [0.4, 0.6]);
        assert_eq!(uvs[2], [0.9, 0.1]);
    } else {
        panic!("Missing ATTRIBUTE_UV_0");
    }

    // 4. Verify reversed triangle winding: [0, 1, 2] -> [0, 2, 1]
    if let Some(Indices::U32(idx)) = mesh.indices() {
        assert_eq!(idx, &[0, 2, 1]);
    } else {
        panic!("Missing Indices::U32");
    }
}

#[test]
fn test_o3d_vertex_colors() {
    let vertices = vec![
        Vertex {
            position: Vec3::new(0.0, 0.0, 0.0),
            normal: Vec3::Y,
            uv: Vec2::ZERO,
            color: Some([255.0, 0.0, 128.0, 255.0]),
        },
        Vertex {
            position: Vec3::new(1.0, 0.0, 0.0),
            normal: Vec3::Y,
            uv: Vec2::X,
            color: Some([0.0, 255.0, 0.0, 255.0]),
        },
        Vertex {
            position: Vec3::new(0.0, 1.0, 0.0),
            normal: Vec3::Y,
            uv: Vec2::Y,
            color: Some([0.0, 0.0, 255.0, 255.0]),
        },
    ];

    let o3d = O3d {
        version: 1,
        header_flags: 0,
        vertices,
        indices: vec![[0, 1, 2]],
        face_materials: vec![0],
        materials: Vec::new(),
        transform: None,
        unknown_chunks: Vec::new(),
        encryption_key: None,
        descrambled: false,
        exact_decoded: false,
    };

    let mesh = o3d_to_bevy_mesh(&o3d);

    if let Some(VertexAttributeValues::Float32x4(colors)) = mesh.attribute(Mesh::ATTRIBUTE_COLOR) {
        assert_eq!(colors.len(), 3);
        assert!((colors[0][0] - 1.0).abs() < 1e-4);
        assert!((colors[0][1] - 0.0).abs() < 1e-4);
        assert!((colors[0][2] - 128.0 / 255.0).abs() < 1e-4);
        assert!((colors[0][3] - 1.0).abs() < 1e-4);
    } else {
        panic!("Expected ATTRIBUTE_COLOR when vertex color is present");
    }
}

#[test]
fn test_x_submesh_conversion() {
    let submesh = XSubMesh {
        name: Some("QuadSubMesh".to_string()),
        vertices: vec![
            XVertex::new(Vec3::new(0.0, 0.0, 1.5), Vec3::Z, Vec2::new(0.0, 0.0)),
            XVertex::new(Vec3::new(1.0, 0.0, 1.5), Vec3::Z, Vec2::new(1.0, 0.0)),
            XVertex::new(Vec3::new(1.0, 1.0, 1.5), Vec3::Z, Vec2::new(1.0, 1.0)),
        ],
        indices: vec![[0, 1, 2]],
        face_materials: vec![0],
        materials: vec![XMaterial::default()],
    };

    let mesh = x_submesh_to_bevy_mesh(&submesh);

    if let Some(VertexAttributeValues::Float32x3(positions)) =
        mesh.attribute(Mesh::ATTRIBUTE_POSITION)
    {
        assert_eq!(positions[0], [0.0, 0.0, -1.5]);
        assert_eq!(positions[1], [1.0, 0.0, -1.5]);
        assert_eq!(positions[2], [1.0, 1.0, -1.5]);
    } else {
        panic!("Missing ATTRIBUTE_POSITION");
    }

    if let Some(Indices::U32(idx)) = mesh.indices() {
        assert_eq!(idx, &[0, 2, 1]);
    } else {
        panic!("Missing Indices::U32");
    }
}

#[test]
fn test_x_mesh_combined_conversion() {
    let submesh1 = XSubMesh {
        name: Some("Sub1".to_string()),
        vertices: vec![
            XVertex::new(Vec3::new(0.0, 0.0, 1.0), Vec3::Y, Vec2::ZERO),
            XVertex::new(Vec3::new(1.0, 0.0, 1.0), Vec3::Y, Vec2::X),
            XVertex::new(Vec3::new(0.0, 1.0, 1.0), Vec3::Y, Vec2::Y),
        ],
        indices: vec![[0, 1, 2]],
        face_materials: vec![0],
        materials: vec![XMaterial::default()],
    };

    let submesh2 = XSubMesh {
        name: Some("Sub2".to_string()),
        vertices: vec![
            XVertex::new(Vec3::new(2.0, 0.0, 2.0), Vec3::Y, Vec2::ZERO),
            XVertex::new(Vec3::new(3.0, 0.0, 2.0), Vec3::Y, Vec2::X),
            XVertex::new(Vec3::new(2.0, 1.0, 2.0), Vec3::Y, Vec2::Y),
        ],
        indices: vec![[0, 1, 2]],
        face_materials: vec![0],
        materials: vec![XMaterial::default()],
    };

    let x_mesh = XMesh {
        frames: vec![XFrame {
            name: Some("FrameA".to_string()),
            transform: None,
            meshes: vec![submesh2],
            children: Vec::new(),
        }],
        top_level_meshes: vec![submesh1],
        global_materials: Vec::new(),
    };

    let submeshes = x_mesh_to_bevy_meshes(&x_mesh);
    assert_eq!(submeshes.len(), 2);

    let combined = x_mesh_to_combined_bevy_mesh(&x_mesh).expect("Failed to combine meshes");

    if let Some(VertexAttributeValues::Float32x3(positions)) =
        combined.attribute(Mesh::ATTRIBUTE_POSITION)
    {
        assert_eq!(positions.len(), 6);
        assert_eq!(positions[0], [0.0, 0.0, -1.0]);
        assert_eq!(positions[3], [2.0, 0.0, -2.0]);
    } else {
        panic!("Missing combined positions");
    }

    if let Some(Indices::U32(idx)) = combined.indices() {
        assert_eq!(idx.len(), 6);
        // First submesh: [0, 2, 1]
        assert_eq!(&idx[0..3], &[0, 2, 1]);
        // Second submesh offset by 3: [3 + 0, 3 + 2, 3 + 1] = [3, 5, 4]
        assert_eq!(&idx[3..6], &[3, 5, 4]);
    } else {
        panic!("Missing combined indices");
    }
}

#[test]
fn test_opts_mesh_plugin_app_registration() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(AssetPlugin {
        watch_for_changes_override: Some(false),
        ..Default::default()
    });
    app.add_plugins(OptsMeshPlugin);

    // Verifies app initializes without panic and asset loaders are registered
    assert!(app.is_plugin_added::<OptsMeshPlugin>());
}
