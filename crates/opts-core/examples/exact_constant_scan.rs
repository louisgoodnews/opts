//! Scratch tool of M-0066: for the memoryless recurrence `s' = (K + data(true position)) mod 8000`
//! finds the constants `K` that decode every given keyed mesh coherently (edge length within 3 %
//! of that file's best). Usage: exact_constant_scan <keyed.o3d>...
use glam::Vec3;
use opts_core::formats::o3d::{exact, parse, O3d};

fn permute(p: [usize; 3], v: Vec3) -> Vec3 {
    let a = [v.x, v.y, v.z];
    Vec3::new(a[p[0]], a[p[1]], a[p[2]])
}

fn edge_sum(m: &O3d, pos: &[Vec3]) -> f64 {
    m.indices
        .iter()
        .map(|t| {
            let p = |i: u32| pos[i as usize];
            let (a, b, c) = (p(t[0]), p(t[1]), p(t[2]));
            f64::from((a - b).length() + (b - c).length() + (c - a).length())
        })
        .sum()
}

fn main() {
    let mut common: Option<Vec<bool>> = None;
    for path in std::env::args().skip(1) {
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let Ok(keyed) = parse(&bytes) else { continue };
        if keyed.vertices.len() < 200 || keyed.indices.is_empty() {
            continue;
        }
        let n = keyed.vertices.len() as i64;
        let mut pos = vec![Vec3::ZERO; keyed.vertices.len()];
        let mut scores = Vec::with_capacity(8000);
        for k in 0..8000i64 {
            let mut s = 0i64;
            for (i, v) in keyed.vertices.iter().enumerate() {
                let q = (n * s).rem_euclid(8000);
                pos[i] = permute(exact::decision(q).position, v.position);
                s = (k + exact::data_term(pos[i])).rem_euclid(8000);
            }
            scores.push(edge_sum(&keyed, &pos));
        }
        let best = scores.iter().cloned().fold(f64::MAX, f64::min);
        let ok: Vec<bool> = scores.iter().map(|s| *s <= best * 1.03).collect();
        println!(
            "{} n={} best {:.1} ok K count {}",
            path.rsplit('/').next().unwrap_or(&path),
            n,
            best,
            ok.iter().filter(|x| **x).count()
        );
        common = Some(match common {
            None => ok,
            Some(c) => c.iter().zip(&ok).map(|(a, b)| *a && *b).collect(),
        });
    }
    let c = common.expect("no usable file");
    let ks: Vec<usize> = c
        .iter()
        .enumerate()
        .filter(|(_, v)| **v)
        .map(|(k, _)| k)
        .collect();
    println!(
        "K good for all: {} values; first 20: {:?}",
        ks.len(),
        &ks[..ks.len().min(20)]
    );
}
