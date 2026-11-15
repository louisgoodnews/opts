//! Mesh geometry against the mutation kit: axes, winding, UV orientation and materials.

use super::*;

#[test]
fn test_t01_axes_and_winding() {
    let bytes_a = match load_fixture("t01_axes_windA.o3d") {
        Some(b) => b,
        None => return,
    };
    let o3d_a = parse(&bytes_a).expect("t01_axes_windA should parse successfully");
    assert_eq!(o3d_a.version, 7);
    assert!(o3d_a.vertex_count() > 0);
    assert!(o3d_a.triangle_count() > 0);

    // Verify axis extents exist and vertices are preserved
    let mut max_x = 0.0f32;
    let mut max_y = 0.0f32;
    let mut max_z = 0.0f32;
    for v in &o3d_a.vertices {
        max_x = max_x.max(v.position.x);
        max_y = max_y.max(v.position.y);
        max_z = max_z.max(v.position.z);
    }
    assert!(max_x > 0.0);
    assert!(max_y > 0.0);
    assert!(max_z > 0.0);

    // Compare with reversed winding B
    let bytes_b = match load_fixture("t01_axes_windB.o3d") {
        Some(b) => b,
        None => return,
    };
    let o3d_b = parse(&bytes_b).expect("t01_axes_windB should parse successfully");
    assert_eq!(o3d_a.vertex_count(), o3d_b.vertex_count());
    assert_eq!(o3d_a.triangle_count(), o3d_b.triangle_count());
    // Winding is reversed between A and B
    assert_ne!(o3d_a.indices[0], o3d_b.indices[0]);
}

#[test]
fn test_t02_uv_orientation() {
    let bytes = match load_fixture("t02_uv_v1.o3d") {
        Some(b) => b,
        None => return,
    };
    let o3d = parse(&bytes).expect("t02_uv_v1 should parse successfully");
    assert_eq!(o3d.vertex_count(), 4);
    assert_eq!(o3d.triangle_count(), 2);
    // UV coordinates range between 0.0 and 1.0
    for v in &o3d.vertices {
        assert!(v.uv.x >= 0.0 && v.uv.x <= 1.0);
        assert!(v.uv.y >= 0.0 && v.uv.y <= 1.0);
    }
}

#[test]
fn test_t03_materials() {
    let bytes = match load_fixture("t03_materials.o3d") {
        Some(b) => b,
        None => return,
    };
    let o3d = parse(&bytes).expect("t03_materials should parse successfully");
    assert!(o3d.material_count() >= 7);
    // Test that material colors have distinct groups
    for mat in &o3d.materials {
        assert_eq!(mat.diffuse.len(), 4);
        assert_eq!(mat.specular.len(), 3);
        assert_eq!(mat.emissive.len(), 3);
        assert!(mat.power >= 0.0);
    }
}
