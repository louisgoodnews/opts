//! Identifiability of the scrambler's texture jitter from a single hinge (P-1013): two triangles
//! `b c x` and `b c d` whose UV mapping is affine across the shared edge satisfy
//! `uv(x) = w . [uv(b), uv(c), uv(d)]` with weights summing to 1. With the stored (jittered) values
//! this is one equation in the four unknown offsets. For a sample of hinges of the plain/keyed twins
//! this counts the offset combinations that satisfy it (up to the common shift that any affine map
//! allows) and whether the true combination is among them.
//! Usage: `uv_hinge_solve <pairs.tsv> [hinges per mesh] [max meshes]`.
#![allow(
    clippy::type_complexity,
    clippy::needless_range_loop,
    clippy::manual_is_multiple_of,
    clippy::needless_borrow,
    unused_mut
)] // research prototype: lint noise, not worth restructuring

use std::collections::HashMap;

use glam::Vec3;
use opts_core::formats::o3d::parse;
use opts_core::formats::o3d::uvfix::jitter_offsets;

fn weights(pb: Vec3, pc: Vec3, pd: Vec3, px: Vec3) -> Option<[f32; 3]> {
    let len = (pc - pb).length();
    if len < 1e-6 {
        return None;
    }
    let e = (pc - pb) / len;
    let rd = pd - pb;
    let sd = rd.dot(e);
    let td = (rd - e * sd).length();
    if td < 1e-3 * len {
        return None;
    }
    let rx = px - pb;
    let sx = rx.dot(e);
    let tx = -(rx - e * sx).length();
    let wc = sx / len - tx * sd / (td * len);
    let wd = tx / td;
    let w = [1.0 - wc - wd, wc, wd];
    w.iter().all(|x| x.abs() <= 8.0).then_some(w)
}

fn main() {
    let mut args = std::env::args().skip(1);
    let text = std::fs::read_to_string(args.next().unwrap()).unwrap();
    let per_mesh: usize = args.next().and_then(|s| s.parse().ok()).unwrap_or(4);
    let max_meshes: usize = args.next().and_then(|s| s.parse().ok()).unwrap_or(400);
    let offs = jitter_offsets();
    let q = |x: f32| (x * 1e4).round() as i64;
    let (mut hinges, mut affine, mut found_true) = (0usize, 0usize, 0usize);
    let mut nsol = [0usize; 6]; // 1, 2, 3-5, 6-20, >20 solutions (excluding the shift family)
    let mut by_class = [[0usize; 3]; 2]; // [5|n][hinges, affine, unique]
    for l in text
        .lines()
        .skip(1)
        .filter(|l| l.split('\t').nth(7) == Some("true"))
        .take(max_meshes)
    {
        let f: Vec<&str> = l.split('\t').collect();
        let (Ok(po), Ok(ko)) = (
            parse(&std::fs::read(f[1]).unwrap()),
            parse(&std::fs::read(f[2]).unwrap()),
        ) else {
            continue;
        };
        let n = po.vertices.len();
        let class = usize::from(n % 5 == 0);
        let mut ids: HashMap<(i64, i64, i64), u32> = HashMap::new();
        let g: Vec<u32> = po
            .vertices
            .iter()
            .map(|v| {
                let k = ids.len() as u32;
                *ids.entry((q(v.position.x), q(v.position.y), q(v.position.z)))
                    .or_insert(k)
            })
            .collect();
        let mut edges: HashMap<(u32, u32), Vec<(usize, usize, usize)>> = HashMap::new();
        for (fi, t) in po.indices.iter().enumerate() {
            for k in 0..3 {
                let (a, b) = (t[k] as usize, t[(k + 1) % 3] as usize);
                let (ga, gb) = (g[a], g[b]);
                if ga != gb {
                    edges
                        .entry((ga.min(gb), ga.max(gb)))
                        .or_default()
                        .push((fi, a, b));
                }
            }
        }
        let mut taken = 0;
        for e in edges.values() {
            if taken >= per_mesh {
                break;
            }
            let [(f1, a1, e1), (f2, a2, e2)] = e[..] else {
                continue;
            };
            if f1 == f2 {
                continue;
            }
            let (b1, c1) = (a1, e1);
            let (b2, c2) = if g[a2] == g[b1] { (a2, e2) } else { (e2, a2) };
            let apex = |fi: usize, p: usize, r: usize| {
                po.indices[fi]
                    .iter()
                    .map(|&v| v as usize)
                    .find(|&v| v != p && v != r)
            };
            let (Some(x), Some(d)) = (apex(f1, b1, c1), apex(f2, b2, c2)) else {
                continue;
            };
            let p = |i: usize| po.vertices[i].position;
            let Some(w) = weights(p(b1), p(c1), p(d), p(x)) else {
                continue;
            };
            // truth residual of the affine prediction
            let tu = |i: usize| [po.vertices[i].uv.x, po.vertices[i].uv.y];
            let su = |i: usize| [ko.vertices[i].uv.x, ko.vertices[i].uv.y];
            // uv(x) predicted from b2, c2, d (as in the decoder)
            let pred_true = [0, 1].map(|k| w[0] * tu(b2)[k] + w[1] * tu(c2)[k] + w[2] * tu(d)[k]);
            let truth_res = [
                (tu(x)[0] - pred_true[0]).abs(),
                (tu(x)[1] - pred_true[1]).abs(),
            ];
            hinges += 1;
            by_class[class][0] += 1;
            taken += 1;
            if truth_res[0] > 2e-4 || truth_res[1] > 2e-4 {
                continue; // not affine across this hinge (seam, bent mapping)
            }
            affine += 1;
            by_class[class][1] += 1;
            // enumerate offsets of b2, c2, d; offset of x must make the equation hold
            let mut sols = 0usize;
            let mut true_in = false;
            let true_off = |i: usize| {
                let (du, dv) = (su(i)[0] - tu(i)[0], su(i)[1] - tu(i)[1]);
                offs.iter()
                    .position(|&(a, b)| (du - a).abs() < 2e-4 && (dv - b).abs() < 2e-4)
            };
            let tr = [true_off(b2), true_off(c2), true_off(d), true_off(x)];
            for ib in 0..offs.len() {
                for ic in 0..offs.len() {
                    for id in 0..offs.len() {
                        let uvb = [su(b2)[0] - offs[ib].0, su(b2)[1] - offs[ib].1];
                        let uvc = [su(c2)[0] - offs[ic].0, su(c2)[1] - offs[ic].1];
                        let uvd = [su(d)[0] - offs[id].0, su(d)[1] - offs[id].1];
                        let pu = w[0] * uvb[0] + w[1] * uvc[0] + w[2] * uvd[0];
                        let pv = w[0] * uvb[1] + w[1] * uvc[1] + w[2] * uvd[1];
                        let (ru, rv) = (su(x)[0] - pu, su(x)[1] - pv);
                        if let Some(ix) = offs
                            .iter()
                            .position(|&(a, b)| (ru - a).abs() < 2e-4 && (rv - b).abs() < 2e-4)
                        {
                            sols += 1;
                            if tr == [Some(ib), Some(ic), Some(id), Some(ix)] {
                                true_in = true;
                            }
                        }
                    }
                }
            }
            found_true += usize::from(true_in);
            let slot = match sols {
                0 | 1 => 0,
                2 => 1,
                3..=5 => 2,
                6..=20 => 3,
                _ => 4,
            };
            nsol[slot] += 1;
            if sols <= 1 {
                by_class[class][2] += 1;
            }
        }
    }
    println!("hinges sampled {hinges}; affine across the hinge {affine} ({:.1}%); true combination among the solutions {found_true}", 100.0 * affine as f64 / hinges.max(1) as f64);
    println!(
        "solutions per affine hinge: <=1: {}, 2: {}, 3-5: {}, 6-20: {}, >20: {}",
        nsol[0], nsol[1], nsol[2], nsol[3], nsol[4]
    );
    for (c, name) in [(0, "other"), (1, "5|n")] {
        println!(
            "{name}: hinges {} affine {} uniquely solved {}",
            by_class[c][0], by_class[c][1], by_class[c][2]
        );
    }
}
