//! O3D vertex colours become mesh colour attributes.

use super::*;

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
