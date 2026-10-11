//! Evaluates joint decoding of keyed duplicates against plain twins (paths on stdin).
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::io::BufRead;

use opts_core::formats::o3d::descramble::{decode_confident_with, decode_joint, DecodeParams};
use opts_core::formats::o3d::{parse, O3d};

fn sig(o: &O3d) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    o.vertices.len().hash(&mut h);
    o.indices.hash(&mut h);
    for v in &o.vertices {
        let mut a = [v.position.x, v.position.y, v.position.z];
        a.sort_by(f32::total_cmp);
        for c in a {
            c.to_bits().hash(&mut h);
        }
    }
    h.finish()
}

fn stats(o: &O3d, truth: &O3d) -> (f32, f32) {
    let (mut lo, mut hi) = (glam::Vec3::splat(f32::MAX), glam::Vec3::splat(f32::MIN));
    for v in &truth.vertices {
        lo = lo.min(v.position);
        hi = hi.max(v.position);
    }
    let tol = 0.01 * (hi - lo).length();
    let n = truth.vertices.len() as f32;
    let pos = o
        .vertices
        .iter()
        .zip(&truth.vertices)
        .filter(|(a, b)| (a.position - b.position).length() <= tol)
        .count() as f32
        / n;
    let uv = o
        .vertices
        .iter()
        .zip(&truth.vertices)
        .filter(|(a, b)| (a.uv - b.uv).abs().max_element() < 2e-4)
        .count() as f32
        / n;
    (pos, uv)
}

fn main() {
    let params = DecodeParams::default();
    let max_k: usize = std::env::var("MAXK")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(4);
    let paths: Vec<String> = std::io::stdin()
        .lock()
        .lines()
        .map_while(Result::ok)
        .collect();
    let mut plain: HashMap<u64, O3d> = HashMap::new();
    let mut keyed: HashMap<u64, Vec<(u64, O3d)>> = HashMap::new();
    for p in &paths {
        let Ok(b) = std::fs::read(p) else { continue };
        if b.len() < 8 || b[0] != 0x84 || b[1] != 0x19 || b[2] < 4 {
            continue;
        }
        let Ok(o) = parse(&b) else { continue };
        if o.vertices.len() < 12 || o.vertices.len() > 12000 {
            continue;
        }
        let s = sig(&o);
        let mut h = std::collections::hash_map::DefaultHasher::new();
        b.hash(&mut h);
        if o.encryption_key.is_some() {
            let e = keyed.entry(s).or_default();
            if !e.iter().any(|(bh, _)| *bh == h.finish()) {
                e.push((h.finish(), o));
            }
        } else {
            plain.entry(s).or_insert(o);
        }
    }
    // per k: (groups, single pos, single uv, joint pos, joint uv, joint accepted, single accepted, single >=98, joint >=98)
    let mut agg: HashMap<usize, [f32; 9]> = HashMap::new();
    for (s, copies) in &keyed {
        let (Some(truth), true) = (plain.get(s), copies.len() >= 2) else {
            continue;
        };
        let k = copies.len().min(max_k);
        let mut single = copies[0].1.clone();
        let rs = decode_confident_with(&mut single, &params);
        let mut joint: Vec<O3d> = copies.iter().take(k).map(|c| c.1.clone()).collect();
        let rj = decode_joint(&mut joint, &params);
        let a = agg.entry(k).or_default();
        a[0] += 1.0;
        if rs.accepted {
            let (p, u) = stats(&single, truth);
            a[1] += p;
            a[2] += u;
            a[6] += 1.0;
            a[7] += (p >= 0.98) as u8 as f32;
        }
        if rj[0].accepted {
            let (p, u) = stats(&joint[0], truth);
            a[3] += p;
            a[4] += u;
            a[5] += 1.0;
            a[8] += (p >= 0.98) as u8 as f32;
        }
    }
    let mut ks: Vec<_> = agg.keys().copied().collect();
    ks.sort();
    for k in ks {
        let a = agg[&k];
        println!("copies {k}: groups {} | single accepted {} exact>=98% {} mean pos {:.3} uv {:.3} | joint accepted {} exact>=98% {} mean pos {:.3} uv {:.3}", a[0], a[6], a[7], a[1] / a[6].max(1.0), a[2] / a[6].max(1.0), a[5], a[8], a[3] / a[5].max(1.0), a[4] / a[5].max(1.0));
    }
}
