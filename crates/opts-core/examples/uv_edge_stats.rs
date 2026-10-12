//! Edge statistics of the plain twins (P-1013): how many welded edges have 1, 2 or more triangles,
//! and how many vertices touch an edge shared by exactly two triangles. Usage: `uv_edge_stats <pairs.tsv>`.
use std::collections::HashMap;

use opts_core::formats::o3d::parse;

fn main() {
    let text = std::fs::read_to_string(std::env::args().nth(1).unwrap()).unwrap();
    let mut hist = [0usize; 6];
    let (mut verts, mut with2, mut with_any_multi, mut isolated_tri) = (0usize, 0usize, 0usize, 0usize);
    for l in text.lines().skip(1) {
        let f: Vec<&str> = l.split('\t').collect();
        if f.len() < 8 || f[7] != "true" {
            continue;
        }
        let Ok(o) = parse(&std::fs::read(f[1]).unwrap()) else { continue };
        let q = |x: f32| (x * 1e4).round() as i64;
        let mut ids: HashMap<(i64, i64, i64), u32> = HashMap::new();
        let g: Vec<u32> = o
            .vertices
            .iter()
            .map(|v| {
                let n = ids.len() as u32;
                *ids.entry((q(v.position.x), q(v.position.y), q(v.position.z))).or_insert(n)
            })
            .collect();
        let mut edges: HashMap<(u32, u32), u32> = HashMap::new();
        for t in &o.indices {
            for k in 0..3 {
                let (a, b) = (g[t[k] as usize], g[t[(k + 1) % 3] as usize]);
                if a != b {
                    *edges.entry((a.min(b), a.max(b))).or_default() += 1;
                }
            }
        }
        for &c in edges.values() {
            hist[(c as usize).min(5)] += 1;
        }
        let mut touch2 = vec![false; o.vertices.len()];
        let mut touch_multi = vec![false; o.vertices.len()];
        let mut tri_neighbours = 0usize;
        for t in &o.indices {
            let mut any = false;
            for k in 0..3 {
                let (a, b) = (g[t[k] as usize], g[t[(k + 1) % 3] as usize]);
                let c = edges.get(&(a.min(b), a.max(b))).copied().unwrap_or(0);
                if c == 2 {
                    touch2[t[k] as usize] = true;
                    touch2[t[(k + 1) % 3] as usize] = true;
                    any = true;
                }
                if c > 2 {
                    touch_multi[t[k] as usize] = true;
                    touch_multi[t[(k + 1) % 3] as usize] = true;
                    any = true;
                }
            }
            if !any {
                tri_neighbours += 1;
            }
        }
        verts += o.vertices.len();
        with2 += touch2.iter().filter(|&&x| x).count();
        with_any_multi += touch_multi.iter().zip(&touch2).filter(|(m, t)| **m && !**t).count();
        isolated_tri += tri_neighbours;
    }
    let tot: usize = hist.iter().sum();
    println!("welded edges: {} ; with 1 triangle {:.1}%, 2: {:.1}%, 3: {:.1}%, 4: {:.1}%, 5+: {:.1}%", tot, 100.0 * hist[1] as f64 / tot as f64, 100.0 * hist[2] as f64 / tot as f64, 100.0 * hist[3] as f64 / tot as f64, 100.0 * hist[4] as f64 / tot as f64, 100.0 * hist[5] as f64 / tot as f64);
    println!("vertices touching a two-triangle edge: {:.1}% ; only >2-triangle edges: {:.1}% ; triangles with no shared edge at all: {}", 100.0 * with2 as f64 / verts as f64, 100.0 * with_any_multi as f64 / verts as f64, isolated_tri);
}
