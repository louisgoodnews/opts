//! X submesh and combined X mesh conversion.

use super::*;

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
