//! Vertex attributes of a submesh: the joints of a skinned model and the winding of its faces.

use super::*;

/// Attaches the joint indices and weights of a skinned model.
pub(super) fn insert_joints(mesh: &mut Mesh, idx: Vec<[u16; 4]>, weights: Vec<[f32; 4]>) {
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_JOINT_INDEX,
        bevy::mesh::VertexAttributeValues::Uint16x4(idx),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT, weights);
}

pub(super) fn align_winding(positions: &[[f32; 3]], normals: &[[f32; 3]], indices: &mut [u32]) {
    let mut agree = 0i64;
    for tri in indices.chunks_exact(3) {
        let p = |i: u32| bevy::math::Vec3::from(positions[i as usize]);
        let n = |i: u32| bevy::math::Vec3::from(normals[i as usize]);
        let face = (p(tri[1]) - p(tri[0])).cross(p(tri[2]) - p(tri[0]));
        let vn = n(tri[0]) + n(tri[1]) + n(tri[2]);
        match face.dot(vn).partial_cmp(&0.0) {
            Some(std::cmp::Ordering::Greater) => agree += 1,
            Some(std::cmp::Ordering::Less) => agree -= 1,
            _ => {}
        }
    }
    if agree < 0 {
        for tri in indices.chunks_exact_mut(3) {
            tri.swap(1, 2);
        }
    }
}
