//! How much do the stored normals of a keyed `.o3d` tell about each vertex's position
//! permutation? On every keyed mesh with a plain copy of the same model: the true position
//! permutation of each vertex (stored -> plain position) against the permutation (ignoring signs)
//! that maps the stored normal onto the plain normal.
//!
//! Usage: `normal_label_eval <omsi root>`
use std::collections::{BTreeMap, HashMap};
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use glam::Vec3;
use opts_core::formats::o3d::{parse, O3d};

/// The scrambler's four permutations, as in `descramble`: `fixed[k] = stored[P[k]]`.
const PERMS: [[usize; 3]; 4] = [[0, 1, 2], [2, 1, 0], [1, 0, 2], [0, 2, 1]];
/// All six axis permutations.
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

fn main() {
    let root = PathBuf::from(std::env::args().nth(1).expect("omsi root"));
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
                if o.vertices.len() < 8 || o.indices.is_empty() {
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
    let pairs: Vec<(PathBuf, PathBuf)> = keyed
        .into_inner()
        .unwrap()
        .into_iter()
        .filter_map(|(h, k)| plain.get(&h).map(|p| (k, p.clone())))
        .collect();
    eprintln!("{} keyed meshes with a plain copy", pairs.len());

    // (position label, normal permutation or "ambiguous"/"none") -> vertices
    let tally: Mutex<BTreeMap<String, usize>> = Mutex::new(BTreeMap::new());
    let next = AtomicUsize::new(0);
    std::thread::scope(|s| {
        for _ in 0..threads {
            s.spawn(|| {
                let mut local: BTreeMap<String, usize> = BTreeMap::new();
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some((kp, pp)) = pairs.get(i) else { break };
                    let (Ok(kb), Ok(pb)) = (std::fs::read(kp), std::fs::read(pp)) else {
                        continue;
                    };
                    let (Ok(k), Ok(p)) = (parse(&kb), parse(&pb)) else {
                        continue;
                    };
                    // Geometric vertex normals of the plain copy (area-weighted face normals).
                    let mut geo = vec![Vec3::ZERO; p.vertices.len()];
                    for f in &p.indices {
                        let [a, b, c] = f.map(|i| i as usize);
                        if a.max(b).max(c) >= p.vertices.len() {
                            continue;
                        }
                        let (pa, pb, pc) = (
                            p.vertices[a].position,
                            p.vertices[b].position,
                            p.vertices[c].position,
                        );
                        let n = (pb - pa).cross(pc - pa);
                        for v in [a, b, c] {
                            geo[v] += n;
                        }
                    }
                    let geometric = std::env::var_os("GEOMETRIC").is_some();
                    for (vi, (sv, pv)) in k.vertices.iter().zip(&p.vertices).enumerate() {
                        let target = if geometric {
                            geo[vi].normalize_or_zero()
                        } else {
                            pv.normal
                        };
                        let pos: Vec<usize> = (0..4)
                            .filter(|&l| {
                                (apply(PERMS[l], sv.position) - pv.position).length() < 1e-4
                            })
                            .collect();
                        if pos.len() != 1 {
                            *local.entry("position ambiguous".into()).or_default() += 1;
                            continue;
                        }
                        let tol = if geometric { 0.05 } else { 2e-3 };
                        let close = |a: Vec3, b: Vec3| (a.abs() - b.abs()).length() < tol;
                        let nrm: Vec<usize> = (0..6)
                            .filter(|&g| {
                                close(apply(PERMS6[g], sv.normal.normalize_or_zero()), target)
                            })
                            .collect();
                        let pl = PERMS[pos[0]];
                        if let [g] = nrm.as_slice() {
                            *local
                                .entry(format!(
                                    "joint position {:?} -> normal {:?}",
                                    pl, PERMS6[*g]
                                ))
                                .or_default() += 1;
                        }
                        let key = match nrm.as_slice() {
                            [] => "normal: no permutation fits".to_string(),
                            [g] if PERMS6[*g] == pl => "normal: same permutation".into(),
                            [_] => "normal: other permutation".into(),
                            many if many.iter().any(|g| PERMS6[*g] == pl) => {
                                "normal: ambiguous, includes the position permutation".into()
                            }
                            _ => "normal: ambiguous, excludes it".into(),
                        };
                        *local.entry(key).or_default() += 1;
                    }
                }
                let mut t = tally.lock().unwrap();
                for (k, n) in local {
                    *t.entry(k).or_default() += n;
                }
            });
        }
    });
    for (k, n) in tally.into_inner().unwrap() {
        println!("{n:9}  {k}");
    }
}
