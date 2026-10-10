//! Finds plain/scrambled twins (same file name and counts) and prints the per-vertex operation
//! of the scrambling: `T <key> <ver> <flags> <n> <i> <perm 0-5|-1> <du> <dv>` for the first
//! `LIMIT` vertices of each pair (all vertices when `LIMIT=0`). Paths on stdin.
use std::collections::HashMap;
use std::io::BufRead;

use opts_core::formats::o3d::parse;

fn main() {
    let limit: usize = std::env::var("LIMIT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1);
    let paths: Vec<String> = std::io::stdin()
        .lock()
        .lines()
        .map_while(Result::ok)
        .collect();
    type Twins = (Vec<String>, Vec<String>);
    let mut by: HashMap<(String, usize, usize), Twins> = HashMap::new();
    for p in &paths {
        let Ok(b) = std::fs::read(p) else { continue };
        if b.len() < 8 || b[0] != 0x84 || b[1] != 0x19 || b[2] < 4 {
            continue;
        }
        let Ok(o) = parse(&b) else { continue };
        let e = by
            .entry((
                p.rsplit('/').next().unwrap_or("").to_lowercase(),
                o.vertices.len(),
                o.indices.len(),
            ))
            .or_default();
        if o.encryption_key.is_some() {
            e.1.push(p.clone())
        } else {
            e.0.push(p.clone())
        }
    }
    let perms: [[usize; 3]; 6] = [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ];
    let mut seen = std::collections::HashSet::new();
    for (_, (plain, enc)) in by {
        let (Some(p), Some(e)) = (plain.first(), enc.first()) else {
            continue;
        };
        let (Ok(pb), Ok(eb)) = (std::fs::read(p), std::fs::read(e)) else {
            continue;
        };
        let (Ok(po), Ok(eo)) = (parse(&pb), parse(&eb)) else {
            continue;
        };
        if po.vertices.len() < 12 {
            continue;
        }
        // same model? sorted position triples must agree at the same index
        let same = po
            .vertices
            .iter()
            .zip(&eo.vertices)
            .filter(|(a, b)| {
                let (mut x, mut y) = (
                    [a.position.x, a.position.y, a.position.z],
                    [b.position.x, b.position.y, b.position.z],
                );
                x.sort_by(|a, b| a.total_cmp(b));
                y.sort_by(|a, b| a.total_cmp(b));
                (0..3).all(|k| (x[k] - y[k]).abs() < 1e-4)
            })
            .count();
        if (same as f32) < 0.95 * po.vertices.len() as f32 {
            continue;
        }
        let sig = (
            eo.encryption_key,
            eo.version,
            eo.header_flags,
            po.vertices
                .iter()
                .take(8)
                .map(|v| v.position.x.to_bits())
                .collect::<Vec<_>>(),
        );
        if !seen.insert(sig) {
            continue;
        }
        let n = po.vertices.len();
        let mut first = vec![usize::MAX; n];
        let mut deg = vec![0usize; n];
        for (k, f) in eo.indices.iter().enumerate() {
            for (c, &v) in f.iter().enumerate() {
                let v = v as usize;
                if v < n {
                    if first[v] == usize::MAX {
                        first[v] = k * 3 + c;
                    }
                    deg[v] += 1;
                }
            }
        }
        for i in 0..(if limit == 0 { n } else { limit.min(n) }) {
            let (a, b) = (&po.vertices[i], &eo.vertices[i]);
            let (pa, pb2) = (
                [a.position.x, a.position.y, a.position.z],
                [b.position.x, b.position.y, b.position.z],
            );
            let cands: Vec<usize> = (0..6)
                .filter(|&pi| (0..3).all(|k| (pb2[k] - pa[perms[pi][k]]).abs() < 1e-6))
                .collect();
            let perm = if cands.len() == 1 {
                cands[0] as i32
            } else {
                -1
            };
            println!("T {:08x} {} {} {n} {i} {perm} {:e} {:e} {:e} {:e} {:e} {:e} {:e} {:e} {:e} {:e} {:e} {:e} {:e} {:e} {:e} {:e} {:e} {:e} {} {} {}", eo.encryption_key.unwrap_or(0), eo.version, eo.header_flags, (b.uv.x - a.uv.x).abs(), (b.uv.y - a.uv.y).abs(), a.position.x, a.position.y, a.position.z, a.normal.x, a.normal.y, a.normal.z, a.uv.x, a.uv.y, b.position.x, b.position.y, b.position.z, b.normal.x, b.normal.y, b.normal.z, b.uv.x, b.uv.y, first[i], deg[i], eo.indices.len());
        }
    }
}
