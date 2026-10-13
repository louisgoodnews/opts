//! How many vertices of plain meshes are duplicates (same position, same or different texture
//! coordinates)? Duplicates would give the decoder equality constraints. Usage: `uv_weld_stats <pairs.tsv>`.
use std::collections::HashMap;

use opts_core::formats::o3d::parse;

fn main() {
    let text = std::fs::read_to_string(std::env::args().nth(1).unwrap()).unwrap();
    let (mut total, mut shared_pos, mut same_uv, mut diff_uv) = (0usize, 0usize, 0usize, 0usize);
    let (mut idx_shared, mut meshes_hi) = (0usize, 0usize);
    let mut meshes = 0usize;
    for l in text.lines().skip(1) {
        let f: Vec<&str> = l.split('\t').collect();
        if f.len() < 8 || f[7] != "true" {
            continue;
        }
        let Ok(o) = parse(&std::fs::read(f[1]).unwrap()) else { continue };
        let q = |x: f32| (x * 1e4).round() as i64;
        let mut by: HashMap<(i64, i64, i64), Vec<usize>> = HashMap::new();
        for (i, v) in o.vertices.iter().enumerate() {
            by.entry((q(v.position.x), q(v.position.y), q(v.position.z))).or_default().push(i);
        }
        let mut used = vec![0u32; o.vertices.len()];
        for t in &o.indices {
            for &i in t {
                if let Some(u) = used.get_mut(i as usize) {
                    *u += 1;
                }
            }
        }
        let n = o.vertices.len();
        total += n;
        meshes += 1;
        let (mut sp, mut su, mut sd) = (0, 0, 0);
        for g in by.values().filter(|g| g.len() > 1) {
            for &i in g {
                sp += 1;
                let uv = o.vertices[i].uv;
                if g.iter().any(|&j| j != i && (o.vertices[j].uv - uv).abs().max_element() < 1e-4) {
                    su += 1;
                } else {
                    sd += 1;
                }
            }
        }
        shared_pos += sp;
        same_uv += su;
        diff_uv += sd;
        idx_shared += used.iter().filter(|&&u| u > 1).count();
        if sp as f32 > 0.5 * n as f32 {
            meshes_hi += 1;
        }
    }
    let p = |x: usize| 100.0 * x as f64 / total as f64;
    println!("meshes {meshes}, vertices {total}");
    println!("vertices sharing their position with another vertex: {:.1}% (same uv {:.1}%, different uv {:.1}%)", p(shared_pos), p(same_uv), p(diff_uv));
    println!("vertices used by more than one triangle (index sharing): {:.1}%", p(idx_shared));
    println!("meshes where more than half of the vertices are position duplicates: {meshes_hi}");
}
