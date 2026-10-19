//! Prints the material table of `.o3d` files and how many faces use each slot.
//! Usage: `o3d_materials <file.o3d>...`
use opts_omsi::formats::o3d::parse;

fn main() {
    for path in std::env::args().skip(1) {
        let Ok(bytes) = std::fs::read(&path) else {
            println!("{path}: unreadable");
            continue;
        };
        let o = match parse(&bytes) {
            Ok(o) => o,
            Err(e) => {
                println!("{path}: {e}");
                continue;
            }
        };
        let mut o = o;
        if std::env::var_os("O3D_DESCRAMBLE").is_some() && o.encryption_key.is_some() {
            opts_omsi::formats::o3d::exact::decode(&mut o);
        }
        let (mut ulo, mut uhi) = ([f32::MAX; 2], [f32::MIN; 2]);
        for v in &o.vertices {
            for (k, c) in [v.uv.x, v.uv.y].into_iter().enumerate() {
                ulo[k] = ulo[k].min(c);
                uhi[k] = uhi[k].max(c);
            }
        }
        println!(
            "  uv range u {:.3}..{:.3} v {:.3}..{:.3}",
            ulo[0], uhi[0], ulo[1], uhi[1]
        );
        let mut faces = vec![0usize; o.materials.len().max(1)];
        let mut out_of_range = 0;
        for m in &o.face_materials {
            match faces.get_mut(usize::from(*m)) {
                Some(f) => *f += 1,
                None => out_of_range += 1,
            }
        }
        println!(
            "{path}: v{} {} vertices, {} triangles, {} materials, scrambled={}",
            o.version,
            o.vertices.len(),
            o.indices.len(),
            o.materials.len(),
            o.encryption_key.is_some()
        );
        for (i, m) in o.materials.iter().enumerate() {
            println!(
                "  slot {i}: faces {:5} texture {:?} diffuse {:?} emissive {:?}",
                faces[i], m.textures, m.diffuse, m.emissive
            );
        }
        if out_of_range > 0 {
            println!("  {out_of_range} faces with a material index beyond the table");
        }
    }
}
