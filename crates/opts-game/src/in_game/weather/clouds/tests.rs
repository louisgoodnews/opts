//! Unit tests for `super` (`clouds.rs`).

use super::*;

#[test]
fn the_texture_is_fixed_to_the_world_and_drifts_downwind() {
    let size = 2000.0;
    // Driving 100 m east moves the disc with the camera, so the texture offset follows.
    let a = uv_transform(Vec2::new(0.0, 0.0), Vec2::ZERO, 0.0, size);
    let b = uv_transform(Vec2::new(100.0, 0.0), Vec2::ZERO, 0.0, size);
    assert!((b.translation.x - a.translation.x - 100.0 / size).abs() < 1e-6);
    // Wind carrying the clouds east (+x) at 6 m/s: after 100 s the pattern sits 600 m further
    // east, so the offset (texture coordinate at the disc centre) has moved the other way.
    let c = uv_transform(Vec2::ZERO, Vec2::new(6.0, 0.0), 100.0, size);
    assert!((c.translation.x - (-600.0f32 / size).rem_euclid(1.0)).abs() < 1e-6);
    // The offset wraps and stays precise far from the origin.
    let far = uv_transform(Vec2::new(1.0e6, 0.0), Vec2::new(1.0, 0.0), 1.0e5, size);
    assert!((0.0..1.0).contains(&far.translation.x));
}

#[test]
fn the_disc_has_a_closed_fan_and_fades_at_its_rim() {
    let mesh = disc_mesh();
    let count = mesh.count_vertices();
    assert_eq!(count, 1 + RINGS * SEGMENTS);
    let Some(Indices::U32(idx)) = mesh.indices() else {
        panic!("indices");
    };
    assert_eq!(idx.len(), 3 * SEGMENTS + 6 * SEGMENTS * (RINGS - 1));
    assert!(idx.iter().all(|i| (*i as usize) < count));
    let bevy::mesh::VertexAttributeValues::Float32x4(colors) =
        mesh.attribute(Mesh::ATTRIBUTE_COLOR).unwrap()
    else {
        panic!("colours");
    };
    assert_eq!(colors[0][3], 1.0);
    assert_eq!(colors[count - 1][3], 0.0);
}
