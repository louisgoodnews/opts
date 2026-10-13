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
            // front faces (counter-clockwise in Bevy) that point away from the centroid: a closed
            // body must show most of its faces outward, otherwise back-face culling hides it
            for (slot, mesh) in &out {
                let pos: Vec<[f32; 3]> = mesh
                    .attribute(bevy::mesh::Mesh::ATTRIBUTE_POSITION)
                    .and_then(|a| a.as_float3())
                    .map(<[_]>::to_vec)
                    .unwrap_or_default();
                let idx: Vec<u32> = mesh
                    .indices()
                    .map(|i| i.iter().map(|x| x as u32).collect())
                    .unwrap_or_default();
                let c = pos.iter().fold(bevy::math::Vec3::ZERO, |a, p| {
                    a + bevy::math::Vec3::from(*p)
                }) / pos.len().max(1) as f32;
                let (mut outward, mut total) = (0usize, 0usize);
                for t in idx.chunks_exact(3) {
                    let p = |i: u32| bevy::math::Vec3::from(pos[i as usize]);
                    let n = (p(t[1]) - p(t[0])).cross(p(t[2]) - p(t[0]));
                    let mid = (p(t[0]) + p(t[1]) + p(t[2])) / 3.0;
                    if n.length() > 1e-9 {
                        total += 1;
                        if n.dot(mid - c) > 0.0 {
                            outward += 1;
                        }
                    }
                }
                println!(
                    "   slot {slot}: {outward}/{total} faces outward ({:.0} %)",
                    100.0 * outward as f32 / total.max(1) as f32
                );
            }
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
