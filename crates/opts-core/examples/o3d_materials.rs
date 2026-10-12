//! Prints per-material triangle counts, diffuse and textures of the `.o3d` files read from stdin.
use std::io::BufRead;

use opts_core::formats::o3d::parse;

fn main() {
    for p in std::io::stdin().lock().lines().map_while(Result::ok) {
        let Ok(b) = std::fs::read(&p) else { continue };
        let Ok(o) = parse(&b) else { continue };
        println!("{p}");
        for (i, m) in o.materials.iter().enumerate() {
            let tris = o.face_materials.iter().filter(|f| **f as usize == i).count();
            let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
            for (t, fm) in o.indices.iter().zip(&o.face_materials) {
                if *fm as usize != i {
                    continue;
                }
                for &vi in t {
                    let v = &o.vertices[vi as usize].position;
                    for (k, c) in [v.x, v.y, v.z].into_iter().enumerate() {
                        lo[k] = lo[k].min(c);
                        hi[k] = hi[k].max(c);
                    }
                }
            }
            println!("  side-zone area {:.2} m2", side_area(&o, i));
            println!(
                "  mat {i}: tris={tris} diffuse={:?} tex={:?} bbox={lo:.2?}..{hi:.2?}",
                m.diffuse,
                m.primary_texture()
            );
        }
    }
}

#[allow(dead_code)]
fn side_area(o: &opts_core::formats::o3d::O3d, mat: usize) -> f32 {
    let mut a = 0.0;
    for (t, fm) in o.indices.iter().zip(&o.face_materials) {
        if *fm as usize != mat {
            continue;
        }
        let p: Vec<_> = t.iter().map(|&i| o.vertices[i as usize].position).collect();
        if p.iter().all(|v| v.x.abs() > 1.2 && v.y > 1.4 && v.y < 2.6) {
            let (u, w) = (
                [p[1].y - p[0].y, p[1].z - p[0].z],
                [p[2].y - p[0].y, p[2].z - p[0].z],
            );
            a += 0.5 * (u[0] * w[1] - u[1] * w[0]).abs();
        }
    }
    a
}
