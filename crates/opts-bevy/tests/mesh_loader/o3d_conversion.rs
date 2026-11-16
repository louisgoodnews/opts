//! O3D positions, normals, UVs and winding are converted.

use super::*;

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
