//! Scratch (not committed): UV range and texture of each material of an .o3d.
fn main() {
    let path = std::env::args().nth(1).expect("path");
    let bytes = std::fs::read(&path).expect("read");
    let m = opts_omsi_binary::formats::o3d::parse(&bytes).expect("parse");
    println!(
        "{path}: {} vertices, {} faces, {} materials",
        m.vertices.len(),
        m.indices.len(),
        m.materials.len()
    );
    for (mi, mat) in m.materials.iter().enumerate() {
        let faces: Vec<usize> = m
            .face_materials
            .iter()
            .enumerate()
            .filter(|(_, &f)| f as usize == mi)
            .map(|(i, _)| i)
            .collect();
        let mut vs: Vec<u32> = faces.iter().flat_map(|&f| m.indices[f]).collect();
        vs.sort_unstable();
        vs.dedup();
        let (mut u0, mut u1, mut v0, mut v1) = (f32::MAX, f32::MIN, f32::MAX, f32::MIN);
        for &v in &vs {
            let uv = m.vertices[v as usize].uv;
            u0 = u0.min(uv.x);
            u1 = u1.max(uv.x);
            v0 = v0.min(uv.y);
            v1 = v1.max(uv.y);
        }
        println!(
            "  mat {mi}: faces {} uv u[{u0:.2},{u1:.2}] v[{v0:.2},{v1:.2}] tex {:?}",
            faces.len(),
            mat.textures
        );
    }
}
