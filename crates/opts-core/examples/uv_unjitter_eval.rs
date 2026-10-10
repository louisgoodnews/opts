#![allow(clippy::type_complexity)]
//! Prototype: undo the scrambler's texture-coordinate jitter by neighbourhood smoothness, evaluated
//! against plain twins (paths on stdin). Positions are taken from the plain twin to isolate the UV step.
use std::collections::HashMap;
use std::io::BufRead;

use opts_core::formats::o3d::parse;

fn main() {
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
    let (mut base_ok, mut fixed_ok, mut total) = (0usize, 0usize, 0usize);
    let (mut wrong_corrections, mut unresolved, mut right_corrections) = (0usize, 0usize, 0usize);
    let mut mesh_fix: Vec<f32> = Vec::new();
    for (_, (plain, enc)) in by {
        let (Some(p), Some(e)) = (plain.first(), enc.first()) else {
            continue;
        };
        let (Ok(po), Ok(eo)) = (
            parse(&std::fs::read(p).unwrap()),
            parse(&std::fs::read(e).unwrap()),
        ) else {
            continue;
        };
        let n = po.vertices.len();
        if !(12..=20000).contains(&n) {
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
        if (same as f32) < 0.95 * n as f32 {
            continue;
        }
        let mut fixed = eo.clone();
        for (v, t) in fixed.vertices.iter_mut().zip(&po.vertices) {
            v.position = t.position; // isolate the UV step: truth positions
        }
        let rep = opts_core::formats::o3d::uvfix::unjitter_uv(&mut fixed);
        let uv: Vec<[f32; 2]> = fixed.vertices.iter().map(|v| [v.uv.x, v.uv.y]).collect();
        let changed = |i: usize| {
            (uv[i][0] - eo.vertices[i].uv.x).abs() > 1e-6
                || (uv[i][1] - eo.vertices[i].uv.y).abs() > 1e-6
        };
        let wrong = |i: usize| {
            (uv[i][0] - po.vertices[i].uv.x).abs() > 2e-4
                || (uv[i][1] - po.vertices[i].uv.y).abs() > 2e-4
        };
        wrong_corrections += (0..n).filter(|&i| changed(i) && wrong(i)).count();
        right_corrections += (0..n).filter(|&i| changed(i) && !wrong(i)).count();
        let _ = rep.corrected;
        unresolved += rep.unresolved;
        let ok =
            |a: [f32; 2], b: glam::Vec2| (a[0] - b.x).abs() < 2e-4 && (a[1] - b.y).abs() < 2e-4;
        let b_ok = (0..n)
            .filter(|&i| {
                ok(
                    [eo.vertices[i].uv.x, eo.vertices[i].uv.y],
                    po.vertices[i].uv,
                )
            })
            .count();
        let f_ok = (0..n).filter(|&i| ok(uv[i], po.vertices[i].uv)).count();
        base_ok += b_ok;
        fixed_ok += f_ok;
        total += n;
        mesh_fix.push(f_ok as f32 / n as f32);
    }
    mesh_fix.sort_by(|a, b| a.total_cmp(b));
    println!("unresolved vertices {unresolved}, changed vertices: {right_corrections} now exact, {wrong_corrections} wrong");
    println!(
        "vertices {total}: uv exact before {:.3}, after {:.3}",
        base_ok as f32 / total as f32,
        fixed_ok as f32 / total as f32
    );
    let m = mesh_fix.len();
    println!(
        "per-mesh exact after: p10 {:.3} p25 {:.3} median {:.3}",
        mesh_fix[m / 10],
        mesh_fix[m / 4],
        mesh_fix[m / 2]
    );
}
