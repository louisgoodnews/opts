//! Decodes each `.o3d` given on the command line the way the loader does and converts it to Bevy
//! meshes without a GPU; prints the submesh count (P-1014 triage of objects "with meshes but
//! none could be loaded").
fn main() {
    for a in std::env::args().skip(1) {
        let b = std::fs::read(&a).unwrap();
        let mut ms = opts_bevy::mesh::parse_mesh_file(std::path::Path::new(&a), &b).unwrap();
        for m in &mut ms {
            let before = (
                m.vertices.len(),
                m.indices.len(),
                m.encryption_key.is_some(),
            );
            let r = if m.encryption_key.is_some() {
                Some(opts_core::formats::o3d::descramble::decode_confident(m))
            } else {
                None
            };
            let out = opts_bevy::mesh::o3d_to_bevy_meshes_by_material(m);
            println!(
                "{a}: v={} t={} keyed={} decode={:?} layout={:?} -> {} submeshes {:?}",
                before.0,
                before.1,
                before.2,
                r.as_ref().map(|r| (r.accepted, r.dropped_faces)),
                m.vertex_layout(),
                out.len(),
                out.iter()
                    .map(|(s, mesh)| (*s, mesh.count_vertices()))
                    .collect::<Vec<_>>()
            );
        }
    }
}
