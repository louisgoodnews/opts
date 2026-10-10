#![allow(clippy::type_complexity)]
//! Visual-quality evaluation of the descrambler on plain/keyed twins (paths on stdin).
//! Reports, per twin, the share of triangles whose three vertices are within 1 % of the model's
//! bounding-box diagonal of the truth, and summary quantiles. `FILTER=<substr>` restricts pairs.
use std::collections::HashMap;
use std::io::BufRead;

use opts_core::formats::o3d::descramble::{labels_score, solved_labels, solved_labels_run};
use opts_core::formats::o3d::parse;

fn main() {
    let filter = std::env::var("FILTER").unwrap_or_default().to_lowercase();
    let paths: Vec<String> = std::io::stdin()
        .lock()
        .lines()
        .map_while(Result::ok)
        .collect();
    let mut by: HashMap<(String, usize, usize), (Vec<String>, Vec<String>)> = HashMap::new();
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
    let (mut opt_fail, mut model_fail) = (0, 0);
    let mut pairs: Vec<(f32, f32)> = Vec::new();
    let mut tri_ok: Vec<f32> = Vec::new();
    let mut vert_ok: Vec<f32> = Vec::new();
    for (_, (plain, enc)) in by {
        let (Some(p), Some(e)) = (plain.first(), enc.first()) else {
            continue;
        };
        if !filter.is_empty() && !e.to_lowercase().contains(&filter) {
            continue;
        }
        let (Ok(po), Ok(eo)) = (
            parse(&std::fs::read(p).unwrap()),
            parse(&std::fs::read(e).unwrap()),
        ) else {
            continue;
        };
        if po.vertices.len() < 12 || po.vertices.len() > 30000 {
            continue;
        }
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
        let (mut lo, mut hi) = (glam::Vec3::splat(f32::MAX), glam::Vec3::splat(f32::MIN));
        for v in &po.vertices {
            lo = lo.min(v.position);
            hi = hi.max(v.position);
        }
        let tol = 0.01 * (hi - lo).length();
        let labels = solved_labels(&eo);
        let ((l0, _), (l1, _)) = (solved_labels_run(&eo, 0), solved_labels_run(&eo, 1));
        let agree = l0.iter().zip(&l1).filter(|(a, b)| a == b).count() as f32 / l0.len() as f32;
        let perms: [[usize; 3]; 4] = [[0, 1, 2], [2, 1, 0], [1, 0, 2], [0, 2, 1]];
        let sol: Vec<glam::Vec3> = eo
            .vertices
            .iter()
            .zip(&labels)
            .map(|(v, &l)| {
                let a = [v.position.x, v.position.y, v.position.z];
                let q = perms[l as usize];
                glam::Vec3::new(a[q[0]], a[q[1]], a[q[2]])
            })
            .collect();
        let truth: Vec<u8> = eo
            .vertices
            .iter()
            .zip(&po.vertices)
            .map(|(v, t)| {
                let a = [v.position.x, v.position.y, v.position.z];
                let tt = [t.position.x, t.position.y, t.position.z];
                (0..4u8)
                    .find(|&l| {
                        let q = perms[l as usize];
                        (0..3).all(|k| (a[q[k]] - tt[k]).abs() < 1e-5)
                    })
                    .unwrap_or(0)
            })
            .collect();
        let (e_found, e_truth) = (labels_score(&eo, &labels), labels_score(&eo, &truth));
        let exact =
            labels.iter().zip(&truth).filter(|(a, b)| a == b).count() as f32 / labels.len() as f32;
        if exact < 0.98 {
            if e_truth > e_found {
                opt_fail += 1
            } else {
                model_fail += 1
            }
        }
        let good: Vec<bool> = sol
            .iter()
            .zip(&po.vertices)
            .map(|(s, t)| (*s - t.position).length() <= tol)
            .collect();
        let truth_ok =
            labels.iter().zip(&truth).filter(|(a, b)| a == b).count() as f32 / labels.len() as f32;
        pairs.push((agree, truth_ok));
        vert_ok.push(good.iter().filter(|g| **g).count() as f32 / good.len() as f32);
        let tris = po.indices.len().max(1);
        let ok = po
            .indices
            .iter()
            .filter(|f| {
                f.iter()
                    .all(|&i| good.get(i as usize).copied().unwrap_or(false))
            })
            .count();
        tri_ok.push(ok as f32 / tris as f32);
    }
    let q = |v: &mut Vec<f32>| {
        v.sort_by(|a, b| a.total_cmp(b));
        let n = v.len().max(1);
        [
            v[n / 10],
            v[n / 4],
            v[n / 2],
            v[3 * n / 4],
            v[(n - 1).min(9 * n / 10)],
        ]
    };
    for thr in [0.9f32, 0.95, 0.98, 0.99, 1.0] {
        let sel: Vec<_> = pairs.iter().filter(|p| p.0 >= thr).collect();
        let good = sel.iter().filter(|p| p.1 >= 0.98).count();
        println!(
            "runs agree >= {thr}: {} meshes, {} truly >=98% exact ({:.1}%)",
            sel.len(),
            good,
            100.0 * good as f32 / sel.len().max(1) as f32
        );
    }
    println!("twins {}; imperfect meshes where truth scores higher (optimiser failure): {opt_fail}, where found scores >= truth (model failure): {model_fail}", tri_ok.len());
    println!(
        "vertices within 1% diag: p10/25/50/75/90 {:?}",
        q(&mut vert_ok)
    );
    println!(
        "triangles fully within 1% diag: p10/25/50/75/90 {:?}",
        q(&mut tri_ok)
    );
    println!(
        "meshes with >=98% triangles ok: {:.3}",
        tri_ok.iter().filter(|t| **t >= 0.98).count() as f32 / tri_ok.len().max(1) as f32
    );
}
