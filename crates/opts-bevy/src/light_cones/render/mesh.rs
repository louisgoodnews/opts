//! The unit cone mesh that every light cone is scaled from.

use super::*;

/// Unit cone: apex at the origin, base ring (radius 1) at z = -1, axis -Z, plus a base cap.
#[must_use]
pub fn unit_cone_mesh() -> Mesh {
    const SEG: usize = 24;
    let mut pos: Vec<[f32; 3]> = Vec::new();
    let mut nor: Vec<[f32; 3]> = Vec::new();
    let mut uv: Vec<[f32; 2]> = Vec::new();
    let mut idx: Vec<u32> = Vec::new();
    // Outward side normals of a cone of half-angle atan(1): (cos, sin, 1) normalised.
    for i in 0..SEG {
        let a0 = i as f32 / SEG as f32 * std::f32::consts::TAU;
        let a1 = (i + 1) as f32 / SEG as f32 * std::f32::consts::TAU;
        let am = 0.5 * (a0 + a1);
        let n = |a: f32| Vec3::new(a.cos(), a.sin(), 1.0).normalize().to_array();
        let base = pos.len() as u32;
        pos.push([0.0, 0.0, 0.0]);
        nor.push(n(am));
        uv.push([0.5, 0.0]);
        pos.push([a0.cos(), a0.sin(), -1.0]);
        nor.push(n(a0));
        uv.push([0.0, 1.0]);
        pos.push([a1.cos(), a1.sin(), -1.0]);
        nor.push(n(a1));
        uv.push([1.0, 1.0]);
        // Counter-clockwise seen from outside.
        idx.extend_from_slice(&[base, base + 1, base + 2]);
    }
    // Cap.
    let c = pos.len() as u32;
    pos.push([0.0, 0.0, -1.0]);
    nor.push([0.0, 0.0, -1.0]);
    uv.push([0.5, 0.5]);
    for i in 0..=SEG {
        let a = i as f32 / SEG as f32 * std::f32::consts::TAU;
        pos.push([a.cos(), a.sin(), -1.0]);
        nor.push([0.0, 0.0, -1.0]);
        uv.push([0.5, 0.5]);
    }
    for i in 0..SEG as u32 {
        idx.extend_from_slice(&[c, c + 2 + i, c + 1 + i]);
    }
    // The CPU copy stays: the bounding box for frustum culling is computed from it.
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_POSITION,
        VertexAttributeValues::Float32x3(pos),
    )
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_NORMAL,
        VertexAttributeValues::Float32x3(nor),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, VertexAttributeValues::Float32x2(uv))
    .with_inserted_indices(Indices::U32(idx))
}
