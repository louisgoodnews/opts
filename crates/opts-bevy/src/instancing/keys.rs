//! Content keys of meshes and materials: equal content gets equal keys, so the pool can share it.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use bevy::mesh::{Indices, Mesh};
use bevy::pbr::StandardMaterial;

/// Hash pair of arbitrary bytes (two differently seeded 64-bit hashes: a collision needs both).
fn hash_pair(parts: &[&[u8]]) -> (u64, u64) {
    let mut a = DefaultHasher::new();
    let mut b = DefaultHasher::new();
    0x9e37_79b9_7f4a_7c15_u64.hash(&mut b);
    for p in parts {
        p.hash(&mut a);
        p.hash(&mut b);
    }
    (a.finish(), b.finish())
}

/// Content key of a mesh: topology, every attribute's bytes and the indices.
#[must_use]
pub fn mesh_key(mesh: &Mesh) -> (u64, u64) {
    let mut chunks: Vec<Vec<u8>> = Vec::new();
    chunks.push(format!("{:?}", mesh.primitive_topology()).into_bytes());
    let mut attrs: Vec<_> = mesh.attributes().collect();
    attrs.sort_by_key(|(a, _)| a.id);
    for (attr, values) in attrs {
        chunks.push(format!("{}:{:?}", attr.name, attr.format).into_bytes());
        chunks.push(values.get_bytes().to_vec());
    }
    match mesh.indices() {
        Some(Indices::U16(v)) => chunks.push(v.iter().flat_map(|i| i.to_le_bytes()).collect()),
        Some(Indices::U32(v)) => chunks.push(v.iter().flat_map(|i| i.to_le_bytes()).collect()),
        None => {}
    }
    let refs: Vec<&[u8]> = chunks.iter().map(Vec::as_slice).collect();
    hash_pair(&refs)
}

/// Content key of a material: its full `Debug` form (colours, texture handles, alpha mode, culling).
#[must_use]
pub fn material_key(material: &StandardMaterial) -> (u64, u64) {
    hash_pair(&[format!("{material:?}").as_bytes()])
}
