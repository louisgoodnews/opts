//! The cloud disc mesh: radial rings with vertex alpha falling to zero at the rim.

use super::*;

/// The disc: radial rings, vertex alpha falling to 0 at the rim, UVs in metres.
pub(super) fn disc_mesh() -> Mesh {
    let mut positions: Vec<[f32; 3]> = vec![[0.0, 0.0, 0.0]];
    let mut colors: Vec<[f32; 4]> = vec![[1.0; 4]];
    for ring in 1..=RINGS {
        let r = RADIUS_M * ring as f32 / RINGS as f32;
        let alpha = ((RADIUS_M - r) / (0.4 * RADIUS_M)).clamp(0.0, 1.0);
        for s in 0..SEGMENTS {
            let a = std::f32::consts::TAU * s as f32 / SEGMENTS as f32;
            positions.push([r * a.cos(), 0.0, r * a.sin()]);
            colors.push([1.0, 1.0, 1.0, alpha]);
        }
    }
    let uvs: Vec<[f32; 2]> = positions.iter().map(|p| [p[0], p[2]]).collect();
    let normals = vec![[0.0, -1.0, 0.0]; positions.len()];
    let at = |ring: usize, s: usize| (1 + (ring - 1) * SEGMENTS + s % SEGMENTS) as u32;
    let mut indices: Vec<u32> = Vec::new();
    for s in 0..SEGMENTS {
        indices.extend([0, at(1, s), at(1, s + 1)]);
    }
    for ring in 1..RINGS {
        for s in 0..SEGMENTS {
            let (a, b) = (at(ring, s), at(ring, s + 1));
            let (c, d) = (at(ring + 1, s), at(ring + 1, s + 1));
            indices.extend([a, c, b, b, c, d]);
        }
    }
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
    .with_inserted_indices(Indices::U32(indices))
}
