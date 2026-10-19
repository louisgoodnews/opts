//! Dumps, for every keyed `.o3d` with a plain copy of the same model, the per-vertex evidence
//! about the scrambler's state: the position permutation (label), what the texture jitter reveals
//! (`q mod 100` from the u-jitter, `2q mod 100` from the v-jitter), and the signed permutation of
//! the normal. One header line per file, one line per vertex (tab separated):
//!
//! ```text
//! F <path> key=<u32> ver=<v> flags=<f> n=<n>
//! V <i> <label|-1> <q mod 100|-1> <2q mod 100|-1> <normal perm 0..5|-1> <sign bits|-1> <stored x y z bits hex> <true u v bits> <true nx ny nz bits> <stored u v bits>
//! ```
//!
//! Usage: `scramble_state_dump <omsi root> <out file> [max vertices per file]`
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use glam::Vec3;
use opts_omsi::formats::o3d::{parse, O3d};

const PERMS: [[usize; 3]; 4] = [[0, 1, 2], [2, 1, 0], [1, 0, 2], [0, 2, 1]];
const PERMS6: [[usize; 3]; 6] = [
    [0, 1, 2],
    [0, 2, 1],
    [1, 0, 2],
    [1, 2, 0],
    [2, 0, 1],
    [2, 1, 0],
];

fn apply(q: [usize; 3], v: Vec3) -> Vec3 {
    let a = [v.x, v.y, v.z];
    Vec3::new(a[q[0]], a[q[1]], a[q[2]])
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            walk(&p, out);
        } else if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("o3d")) {
            out.push(p);
        }
    }
}

fn model_hash(o: &O3d) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    o.vertices.len().hash(&mut h);
    o.indices.hash(&mut h);
    for v in &o.vertices {
        let mut x = [v.position.x, v.position.y, v.position.z];
        x.sort_by(f32::total_cmp);
        for c in x {
            c.to_bits().hash(&mut h);
        }
    }
    h.finish()
}

/// `d = (m/100)^2` for an integer `m` in `1..100`: that `m`, or `None`.
fn jitter_code(d: f32) -> Option<i32> {
    if d.abs() < 1e-6 {
        return None;
    }
    if d < 0.0 {
        return Some(-2); // negative: not a jitter of the known form
    }
    let m = (d as f64).sqrt() * 100.0;
    let r = m.round();
    if (m - r).abs() < 0.02 && (1.0..100.0).contains(&r) {
        Some(r as i32)
    } else {
        Some(-3) // positive but not of the form
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let root = PathBuf::from(args.next().expect("omsi root"));
    let out_path = PathBuf::from(args.next().expect("out file"));
    let max_n: usize = args
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(usize::MAX);
    let mut files = Vec::new();
    walk(&root, &mut files);
    let plain: Mutex<HashMap<u64, PathBuf>> = Mutex::new(HashMap::new());
    let keyed: Mutex<Vec<(u64, PathBuf)>> = Mutex::new(Vec::new());
    let next = AtomicUsize::new(0);
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    std::thread::scope(|s| {
        for _ in 0..threads {
            s.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::Relaxed);
                let Some(p) = files.get(i) else { break };
                let Ok(b) = std::fs::read(p) else { continue };
                let Ok(o) = parse(&b) else { continue };
                if o.vertices.len() < 4 || o.indices.is_empty() {
                    continue;
                }
                let h = model_hash(&o);
                if o.encryption_key.is_some() {
                    keyed.lock().unwrap().push((h, p.clone()));
                } else {
                    plain.lock().unwrap().entry(h).or_insert_with(|| p.clone());
                }
            });
        }
    });
    let plain = plain.into_inner().unwrap();
    let mut pairs: Vec<(PathBuf, PathBuf)> = keyed
        .into_inner()
        .unwrap()
        .into_iter()
        .filter_map(|(h, k)| plain.get(&h).map(|p| (k, p.clone())))
        .collect();
    pairs.sort();
    eprintln!("{} keyed meshes with a plain copy", pairs.len());
    let mut out = std::io::BufWriter::new(std::fs::File::create(&out_path).expect("create out"));
    let close =
        |a: Vec3, b: Vec3| (a - b).abs().max_element() <= 1e-5 * (1.0 + a.abs().max_element());
    for (kp, pp) in &pairs {
        let (Ok(kb), Ok(pb)) = (std::fs::read(kp), std::fs::read(pp)) else {
            continue;
        };
        let (Ok(k), Ok(p)) = (parse(&kb), parse(&pb)) else {
            continue;
        };
        if k.vertices.len() > max_n {
            continue;
        }
        writeln!(
            out,
            "F {} key={} ver={} flags={} n={}",
            kp.strip_prefix(&root).unwrap_or(kp).display(),
            k.encryption_key.unwrap_or(0),
            k.version,
            k.header_flags,
            k.vertices.len()
        )
        .unwrap();
        // Face-side properties of each vertex (identical in every copy of the model).
        let nv = k.vertices.len();
        let mut valence = vec![0u32; nv];
        let mut first_face = vec![-1i64; nv];
        let mut first_mat = vec![-1i64; nv];
        for (fi, f) in k.indices.iter().enumerate() {
            for &vi in f {
                if (vi as usize) < nv {
                    valence[vi as usize] += 1;
                    if first_face[vi as usize] < 0 {
                        first_face[vi as usize] = fi as i64;
                        first_mat[vi as usize] =
                            k.face_materials.get(fi).map_or(-1, |m| i64::from(*m));
                    }
                }
            }
        }
        for (i, (sv, pv)) in k.vertices.iter().zip(&p.vertices).enumerate() {
            let labels: Vec<usize> = (0..4)
                .filter(|&l| close(apply(PERMS[l], sv.position), pv.position))
                .collect();
            let label = if labels.len() == 1 {
                labels[0] as i32
            } else {
                -1
            };
            let qu = jitter_code(sv.uv.x - pv.uv.x).unwrap_or(-1);
            let qv = jitter_code(sv.uv.y - pv.uv.y).unwrap_or(-1);
            // Signed permutation of the normal: true = sign ∘ perm(stored).
            let mut nperm = -1;
            let mut nsign = -1;
            let sn = sv.normal;
            let tn = pv.normal;
            if sn.length() > 1e-6 && tn.length() > 1e-6 {
                let mut hits = Vec::new();
                for (g, perm) in PERMS6.iter().enumerate() {
                    let pn = apply(*perm, sn);
                    for sgn in 0..8u32 {
                        let s = Vec3::new(
                            if sgn & 1 != 0 { -1.0 } else { 1.0 },
                            if sgn & 2 != 0 { -1.0 } else { 1.0 },
                            if sgn & 4 != 0 { -1.0 } else { 1.0 },
                        );
                        if close(pn * s, tn) {
                            hits.push((g as i32, sgn as i32));
                        }
                    }
                }
                if hits.len() == 1 {
                    nperm = hits[0].0;
                    nsign = hits[0].1;
                } else if !hits.is_empty() {
                    nperm = -2; // ambiguous (zero components)
                }
            }
            writeln!(
                out,
                "V {i} {label} {qu} {qv} {nperm} {nsign} {:08x} {:08x} {:08x} {:08x} {:08x} {:08x} {:08x} {:08x} {:08x} {:08x} {} {} {}",
                sv.position.x.to_bits(),
                sv.position.y.to_bits(),
                sv.position.z.to_bits(),
                pv.uv.x.to_bits(),
                pv.uv.y.to_bits(),
                pv.normal.x.to_bits(),
                pv.normal.y.to_bits(),
                pv.normal.z.to_bits(),
                sv.uv.x.to_bits(),
                sv.uv.y.to_bits(),
                valence[i],
                first_face[i],
                first_mat[i]
            )
            .unwrap();
        }
    }
}
