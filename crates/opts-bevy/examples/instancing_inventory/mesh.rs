//! Hashing and reading of one mesh file for the inventory: its material slots and counts.

use std::collections::hash_map::DefaultHasher;
use std::collections::BTreeSet;
use std::hash::{Hash, Hasher};
use std::path::Path;

/// One material slot of a mesh file that has triangles.
#[derive(Clone)]
pub(super) struct Slot {
    pub(super) texture: String,
    /// `.o3d` material numbers, quantised.
    pub(super) numbers: [i32; 11],
}

pub(super) struct MeshInfo {
    /// Hash of the file bytes (equal for the same mesh copied between add-ons).
    pub(super) content: u64,
    pub(super) slots: Vec<Slot>,
    pub(super) triangles: usize,
    pub(super) vertices: usize,
    pub(super) scrambled: bool,
}
pub(super) fn base_name(p: &str) -> String {
    p.replace('\\', "/")
        .rsplit('/')
        .next()
        .unwrap_or("")
        .to_lowercase()
}

fn q(v: f32) -> i32 {
    (v * 1000.0).round() as i32
}

pub(super) fn mesh_info(path: &Path) -> Option<MeshInfo> {
    let bytes = std::fs::read(path).ok()?;
    let mut h = DefaultHasher::new();
    bytes.hash(&mut h);
    let models = opts_bevy::mesh::parse_mesh_file(path, &bytes).ok()?;
    let mut slots = Vec::new();
    let (mut triangles, mut vertices) = (0, 0);
    let mut scrambled = false;
    for m in &models {
        triangles += m.indices.len();
        vertices += m.vertices.len();
        scrambled |= matches!(
            m.vertex_layout(),
            opts_core::formats::o3d::VertexLayout::Unreadable
        );
        let unreadable = matches!(
            m.vertex_layout(),
            opts_core::formats::o3d::VertexLayout::Unreadable
        );
        // Scrambled files: the per-face material ids are not trusted; assume every table entry
        // is used (the decoder of the game recovers the real split).
        let used: BTreeSet<usize> = if unreadable {
            (0..m.materials.len().max(1)).collect()
        } else {
            (0..m.indices.len())
                .map(|i| m.face_materials.get(i).copied().unwrap_or(0) as usize)
                .collect()
        };
        for s in used {
            let (texture, numbers) = match m.materials.get(s) {
                Some(mat) => (
                    mat.primary_texture().map(base_name).unwrap_or_default(),
                    [
                        q(mat.diffuse[0]),
                        q(mat.diffuse[1]),
                        q(mat.diffuse[2]),
                        q(mat.diffuse[3]),
                        q(mat.specular[0]),
                        q(mat.specular[1]),
                        q(mat.specular[2]),
                        q(mat.emissive[0]),
                        q(mat.emissive[1]),
                        q(mat.emissive[2]),
                        q(mat.power),
                    ],
                ),
                None => (String::new(), [1000; 11]),
            };
            slots.push(Slot { texture, numbers });
        }
    }
    Some(MeshInfo {
        content: h.finish(),
        slots,
        triangles,
        vertices,
        scrambled,
    })
}
