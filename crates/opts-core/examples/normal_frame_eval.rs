//! Can the stored normals put the connected parts of a decoded keyed mesh into the right frame?
//! For every keyed `.o3d` with a plain copy of the same model: decode it (`decode_confident`, with
//! the vertex-0 anchor), then for every connected part try the axis permutations that keep every
//! vertex reachable from its stored position, and score each by how many vertices then agree with
//! [`NORMAL_PERMS`] (the stored normal, permuted as the vertex's label allows, matches the
//! geometric normal). A part is re-framed when another permutation beats the decoded one by
//! `MARGIN` (share of the part's vertices, env, default 0.2). Counts exact decodes before and
//! after, and how many exact ones were broken.
//!
//! Usage: `normal_frame_eval <omsi root>`
use std::collections::{BTreeMap, HashMap};
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use glam::Vec3;
use opts_core::formats::o3d::descramble::{
    decode_confident, decode_confident_with, DecodeParams, NORMAL_PERMS,
};
use opts_core::formats::o3d::{parse, O3d};

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

fn parts(o: &O3d) -> Vec<usize> {
    let n = o.vertices.len();
    let mut up: Vec<usize> = (0..n).collect();
    fn find(up: &mut [usize], mut i: usize) -> usize {
        while up[i] != i {
            up[i] = up[up[i]];
            i = up[i];
        }
        i
    }
    for t in &o.indices {
        let r: Vec<usize> = t
            .iter()
            .filter(|&&i| (i as usize) < n)
            .map(|&i| find(&mut up, i as usize))
            .collect();
        for w in r.windows(2) {
            let (a, b) = (find(&mut up, w[0]), find(&mut up, w[1]));
            up[b] = a;
        }
    }
    (0..n).map(|i| find(&mut up, i)).collect()
}

/// Geometric vertex normals (area-weighted face normals) of `pos` under the faces of `o`.
fn geo_normals(o: &O3d, pos: &[Vec3]) -> Vec<Vec3> {
    let mut geo = vec![Vec3::ZERO; pos.len()];
    for f in &o.indices {
        let [a, b, c] = f.map(|i| i as usize);
        if a.max(b).max(c) >= pos.len() {
            continue;
        }
        let n = (pos[b] - pos[a]).cross(pos[c] - pos[a]);
        for v in [a, b, c] {
            geo[v] += n;
        }
    }
    geo.into_iter().map(|g| g.normalize_or_zero()).collect()
}

/// Whether vertex `stored` (with stored normal) at true position `p` agrees with the normal table.
fn agrees(stored: Vec3, normal: Vec3, p: Vec3, geo: Vec3) -> bool {
    let close = |a: Vec3, b: Vec3| (a - b).length() < 1e-4;
    let Some(l) = (0..4).find(|&l| close(apply(PERMS[l], stored), p)) else {
        return false;
    };
    let n = normal.normalize_or_zero().abs();
    NORMAL_PERMS[l]
        .iter()
        .any(|q| (apply(*q, n) - geo.abs()).length() < 0.05)
}

fn identical(a: &[Vec3], b: &O3d) -> bool {
    a.iter()
        .zip(&b.vertices)
        .all(|(x, y)| (*x - y.position).length() < 1e-3)
}

fn main() {
    let root = PathBuf::from(std::env::args().nth(1).expect("omsi root"));
    let margin: f32 = std::env::var("MARGIN")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0.2);
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
    let tally: Mutex<BTreeMap<String, usize>> = Mutex::new(BTreeMap::new());
    let next = AtomicUsize::new(0);
    std::thread::scope(|s| {
        for _ in 0..threads {
            s.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::Relaxed);
                let Some((kp, pp)) = pairs.get(i) else { break };
                let (Ok(kb), Ok(pb)) = (std::fs::read(kp), std::fs::read(pp)) else {
                    continue;
                };
                let (Ok(k), Ok(p)) = (parse(&kb), parse(&pb)) else {
                    continue;
                };
                if std::env::var_os("SELECT").is_some() {
                    // Decode with the default and with the label-aware normal term; keep the
                    // result whose vertices agree more often with the normal table.
                    let alt_params = DecodeParams {
                        w_normal_label: std::env::var("SELECT_W")
                            .ok()
                            .and_then(|v| v.parse().ok())
                            .unwrap_or(0.5),
                        ..DecodeParams::default()
                    };
                    let mut a = k.clone();
                    let mut b = k.clone();
                    let ok_a = decode_confident(&mut a).accepted;
                    let ok_b = decode_confident_with(&mut b, &alt_params).accepted;
                    let share = |d: &O3d| {
                        let pos: Vec<Vec3> = d.vertices.iter().map(|v| v.position).collect();
                        let geo = geo_normals(d, &pos);
                        (0..pos.len())
                            .filter(|&v| {
                                agrees(k.vertices[v].position, k.vertices[v].normal, pos[v], geo[v])
                            })
                            .count() as f32
                            / pos.len().max(1) as f32
                    };
                    let ex = |ok: bool, d: &O3d| {
                        ok && identical(
                            &d.vertices.iter().map(|v| v.position).collect::<Vec<_>>(),
                            &p,
                        )
                    };
                    let (sa, sb) = (
                        if ok_a { share(&a) } else { -1.0 },
                        if ok_b { share(&b) } else { -1.0 },
                    );
                    let pick_b = sb > sa + 1e-6;
                    let (ea, eb) = (ex(ok_a, &a), ex(ok_b, &b));
                    let picked = if pick_b { eb } else { ea };
                    let key = format!(
                        "default {} | alt {} | picked {}",
                        if ea { "exact" } else { "wrong" },
                        if eb { "exact" } else { "wrong" },
                        if picked { "exact" } else { "wrong" }
                    );
                    *tally.lock().unwrap().entry(key).or_default() += 1;
                    continue;
                }
                let mut d = k.clone();
                if !decode_confident(&mut d).accepted {
                    continue;
                }
                let before: Vec<Vec3> = d.vertices.iter().map(|v| v.position).collect();
                let mut pos = before.clone();
                let part = parts(&d);
                let mut groups: HashMap<usize, Vec<usize>> = HashMap::new();
                for (vi, r) in part.iter().enumerate() {
                    groups.entry(*r).or_default().push(vi);
                }
                let mut reframed = 0;
                for vs in groups.values() {
                    let reach = |g: usize| {
                        vs.iter().all(|&v| {
                            let t = apply(PERMS6[g], before[v]);
                            (0..4).any(|l| {
                                (apply(PERMS[l], k.vertices[v].position) - t).length() < 1e-4
                            })
                        })
                    };
                    let score = |g: usize| -> usize {
                        let mut cand = before.clone();
                        for &v in vs {
                            cand[v] = apply(PERMS6[g], before[v]);
                        }
                        let geo = geo_normals(&d, &cand);
                        vs.iter()
                            .filter(|&&v| {
                                agrees(
                                    k.vertices[v].position,
                                    k.vertices[v].normal,
                                    cand[v],
                                    geo[v],
                                )
                            })
                            .count()
                    };
                    let base = score(0);
                    let best = (1..6)
                        .filter(|&g| reach(g))
                        .map(|g| (g, score(g)))
                        .max_by_key(|x| x.1);
                    if let Some((g, sc)) = best {
                        if sc as f32 > base as f32 + margin * vs.len() as f32 {
                            for &v in vs {
                                pos[v] = apply(PERMS6[g], before[v]);
                            }
                            reframed += 1;
                        }
                    }
                }
                let (b, a) = (identical(&before, &p), identical(&pos, &p));
                let key = format!(
                    "before {} -> after {}{}",
                    if b { "exact" } else { "wrong" },
                    if a { "exact" } else { "wrong" },
                    if reframed > 0 { " (re-framed)" } else { "" }
                );
                *tally.lock().unwrap().entry(key).or_default() += 1;
            });
        }
    });
    for (k, n) in tally.into_inner().unwrap() {
        println!("{n:6}  {k}");
    }
}
