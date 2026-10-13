#![allow(clippy::all, unused)]
#![allow(unused_mut, clippy::type_complexity, clippy::manual_is_multiple_of)]
//! Prototype (P-1013 B1): how much does a chain model over the vertex order add to the present
//! decoder? Trains transition and confusion tables on even pairs of `uv_truth`'s `vertices.tsv`
//! and scores the forward-backward posterior on odd pairs. Usage: `uv_chain_proto <vertices.tsv>`.
use std::collections::HashMap;

fn km(du: f32, dv: f32) -> (i32, i32) {
    let k = if du.abs() > 1e-6 {
        (du.max(0.0) * 400.0).sqrt().round() as i32
    } else {
        0
    };
    let m = if dv.abs() > 1e-6 {
        (dv.max(0.0) * 2500.0).sqrt().round() as i32
    } else {
        0
    };
    (k, m)
}

struct V {
    pair: u32,
    n: u32,
    truth: usize,
    seen: usize,
}

fn main() {
    let path = std::env::args().nth(1).unwrap();
    let text = std::fs::read_to_string(path).unwrap();
    let mut ids: HashMap<(i32, i32), usize> = HashMap::new();
    let mut verts: Vec<V> = Vec::new();
    let mut id_of = |km: (i32, i32), ids: &mut HashMap<(i32, i32), usize>| {
        let l = ids.len();
        *ids.entry(km).or_insert(l)
    };
    for line in text.lines() {
        let f: Vec<&str> = line.split('\t').collect();
        if f.len() < 18 {
            continue;
        }
        let p = |i: usize| f[i].parse::<f32>().unwrap_or(f32::NAN);
        let (du, dv, sdu, sdv) = (p(9), p(10), p(16), p(17));
        if sdu.is_nan() {
            continue;
        }
        let t = km(du, dv);
        let s = km((du - sdu).max(0.0), (dv - sdv).max(0.0));
        // a wrongly shifted vertex has a negative removed offset: treated as "other" = (-1,-1)
        let s = if du - sdu < -1e-6 || dv - sdv < -1e-6 {
            (-1, -1)
        } else {
            s
        };
        let (ti, si) = (id_of(t, &mut ids), id_of(s, &mut ids));
        verts.push(V {
            pair: f[0].parse().unwrap(),
            n: f[2].parse().unwrap(),
            truth: ti,
            seen: si,
        });
    }
    let s = ids.len();
    eprintln!("{} vertices, {} states (incl. other)", verts.len(), s);
    // group by pair
    let mut meshes: Vec<(u32, u32, Vec<(usize, usize)>)> = Vec::new();
    for v in &verts {
        match meshes.last_mut() {
            Some(m) if m.0 == v.pair => m.2.push((v.truth, v.seen)),
            _ => meshes.push((v.pair, v.n, vec![(v.truth, v.seen)])),
        }
    }
    let train = |m: &&(u32, u32, Vec<(usize, usize)>)| m.0 % 2 == 0;
    let class = |n: u32| usize::from(n % 5 == 0);
    let mut trans = vec![vec![vec![0.5f64; s]; s]; 2];
    let mut start = vec![vec![0.5f64; s]; 2];
    let mut emit = vec![vec![0.05f64; s]; s];
    for m in meshes.iter().filter(train) {
        let c = class(m.1);
        start[c][m.2[0].0] += 1.0;
        for w in m.2.windows(2) {
            trans[c][w[0].0][w[1].0] += 1.0;
        }
        for &(t, o) in &m.2 {
            emit[t][o] += 1.0;
        }
    }
    for c in 0..2 {
        for a in 0..s {
            let z: f64 = trans[c][a].iter().sum();
            trans[c][a].iter_mut().for_each(|x| *x /= z);
        }
        let z: f64 = start[c].iter().sum();
        start[c].iter_mut().for_each(|x| *x /= z);
    }
    for a in 0..s {
        let z: f64 = emit[a].iter().sum();
        emit[a].iter_mut().for_each(|x| *x /= z);
    }
    let (mut tot, mut base, mut post, mut hi_conf, mut hi_conf_ok) =
        (0usize, 0usize, 0usize, 0usize, 0usize);
    let mut by_class = [[0usize; 3]; 2];
    for m in meshes.iter().filter(|m| !train(&m)) {
        let c = class(m.1);
        let len = m.2.len();
        let mut alpha = vec![vec![0.0f64; s]; len];
        let mut beta = vec![vec![1.0f64; s]; len];
        for a in 0..s {
            alpha[0][a] = start[c][a] * emit[a][m.2[0].1];
        }
        let norm = |v: &mut Vec<f64>| {
            let z: f64 = v.iter().sum();
            if z > 0.0 {
                v.iter_mut().for_each(|x| *x /= z);
            }
        };
        norm(&mut alpha[0]);
        for i in 1..len {
            let (prev, cur) = alpha.split_at_mut(i);
            let (prev, cur) = (&prev[i - 1], &mut cur[0]);
            for b in 0..s {
                let mut acc = 0.0;
                for a in 0..s {
                    acc += prev[a] * trans[c][a][b];
                }
                cur[b] = acc * emit[b][m.2[i].1];
            }
            norm(cur);
        }
        for i in (0..len - 1).rev() {
            let (cur, next) = beta.split_at_mut(i + 1);
            let (cur, next) = (&mut cur[i], &next[0]);
            for a in 0..s {
                let mut acc = 0.0;
                for b in 0..s {
                    acc += trans[c][a][b] * emit[b][m.2[i + 1].1] * next[b];
                }
                cur[a] = acc;
            }
            norm(cur);
        }
        for i in 0..len {
            let mut p: Vec<f64> = (0..s).map(|a| alpha[i][a] * beta[i][a]).collect();
            norm(&mut p);
            let (best, pb) = p
                .iter()
                .enumerate()
                .fold((0, 0.0), |b, (k, &x)| if x > b.1 { (k, x) } else { b });
            tot += 1;
            base += usize::from(m.2[i].0 == m.2[i].1);
            let ok = best == m.2[i].0;
            post += usize::from(ok);
            by_class[c][0] += 1;
            by_class[c][1] += usize::from(m.2[i].0 == m.2[i].1);
            by_class[c][2] += usize::from(ok);
            if pb > 0.95 {
                hi_conf += 1;
                hi_conf_ok += usize::from(ok);
            }
        }
    }
    println!(
        "test vertices {tot}: decoder exact {:.4}, chain posterior exact {:.4}",
        base as f64 / tot as f64,
        post as f64 / tot as f64
    );
    for c in 0..2 {
        println!(
            "class {}: n {} decoder {:.4} posterior {:.4}",
            if c == 1 { "5|n" } else { "other" },
            by_class[c][0],
            by_class[c][1] as f64 / by_class[c][0] as f64,
            by_class[c][2] as f64 / by_class[c][0] as f64
        );
    }
    println!(
        "posterior >0.95: {} vertices ({:.3}), of which right {:.4}",
        hi_conf,
        hi_conf as f64 / tot as f64,
        hi_conf_ok as f64 / hi_conf.max(1) as f64
    );
}
