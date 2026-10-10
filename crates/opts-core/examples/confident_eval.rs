#![allow(clippy::type_complexity)]
//! Evaluates `decode_confident` on plain/keyed twins (paths on stdin): coverage, precision, UV exactness.
use std::collections::HashMap;
use std::io::BufRead;

use opts_core::formats::o3d::descramble::{decode_confident_with, DecodeParams};
use opts_core::formats::o3d::parse;

fn main() {
    let params = DecodeParams::default().with_overrides(|k| std::env::var(k).ok());
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
    let (mut twins, mut accepted, mut good_pos, mut good_both) = (0, 0, 0, 0);
    let (mut uv_ok, mut uv_tot) = (0usize, 0usize);
    let mut rejected_good = 0;
    for (_, (plain, enc)) in by {
        let (Some(p), Some(e)) = (plain.first(), enc.first()) else {
            continue;
        };
        let (Ok(po), Ok(mut eo)) = (
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
        twins += 1;
        let enc0 = eo.clone();
        let rep = decode_confident_with(&mut eo, &params);
        // positions: decode_confident keeps vertex order and only drops triangles
        let (mut lo, mut hi) = (glam::Vec3::splat(f32::MAX), glam::Vec3::splat(f32::MIN));
        for v in &po.vertices {
            lo = lo.min(v.position);
            hi = hi.max(v.position);
        }
        let tol = 0.01 * (hi - lo).length();
        let pos_ok = eo
            .vertices
            .iter()
            .zip(&po.vertices)
            .filter(|(a, b)| (a.position - b.position).length() <= tol)
            .count() as f32
            / po.vertices.len() as f32;
        if rep.accepted {
            accepted += 1;
            if pos_ok >= 0.98 {
                good_pos += 1;
            }
            let u = eo
                .vertices
                .iter()
                .zip(&po.vertices)
                .filter(|(a, b)| (a.uv - b.uv).abs().max_element() < 2e-4)
                .count();
            uv_ok += u;
            uv_tot += po.vertices.len();
            if pos_ok >= 0.98 && u as f32 >= 0.9 * po.vertices.len() as f32 {
                good_both += 1;
            }
        } else {
            let _ = enc0;
            // would a single plain run have been right? (coverage loss estimate)
            rejected_good += 0;
        }
    }
    println!("twins {twins}, accepted {accepted} ({:.1}%), accepted with >=98% positions right {good_pos} ({:.1}% of accepted), with also >=90% uv exact {good_both}", 100.0 * accepted as f32 / twins as f32, 100.0 * good_pos as f32 / accepted.max(1) as f32);
    println!(
        "uv exact among accepted vertices: {:.3}",
        uv_ok as f32 / uv_tot.max(1) as f32
    );
    let _ = rejected_good;
}
