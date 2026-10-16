//! Checks the vertex-0 rule (`descramble::vertex0_label`, o3d.md Addendum 4 Update 3 and
//! Addendum 12) on every keyed `.o3d` that has a plain copy of the same model (same face list,
//! same sorted coordinate triple at every vertex): is the permutation that maps the stored
//! vertex 0 onto the plain one the one the rule names? With `--decode` the copy is also decoded
//! (`decode_confident`, which applies the vertex-0 frame anchor) and compared with the plain one.
//!
//! Usage: `vertex0_anchor_eval <omsi root> [--decode]`
use std::collections::{BTreeMap, HashMap};
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use glam::Vec3;
use opts_core::formats::o3d::descramble::{decode_confident, vertex0_label};
use opts_core::formats::o3d::{parse, O3d};

/// The scrambler's four permutations, as in `descramble`: `fixed[k] = stored[P[k]]`.
const PERMS: [[usize; 3]; 4] = [[0, 1, 2], [2, 1, 0], [1, 0, 2], [0, 2, 1]];

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
    let mut args = std::env::args().skip(1);
    let root = PathBuf::from(args.next().expect("omsi root"));
    let decode = args.next().as_deref() == Some("--decode");
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
    let bump = |k: String| *tally.lock().unwrap().entry(k).or_default() += 1;
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
                let close = |a: Vec3, b: Vec3| (a - b).length() < 1e-4;
                let truth: Vec<usize> = (0..4)
                    .filter(|&l| {
                        close(
                            apply(PERMS[l], k.vertices[0].position),
                            p.vertices[0].position,
                        )
                    })
                    .collect();
                let seed = match (k.header_flags & 2 != 0, k.encryption_key == Some(0)) {
                    (true, _) => "alternative seed",
                    (false, true) => "key 0",
                    (false, false) => "standard",
                };
                let rule_ok = vertex0_label(&k).is_some_and(|l| truth.contains(&l));
                bump(format!(
                    "rule {seed:16} {}",
                    if rule_ok { "right" } else { "wrong" }
                ));
                if !decode {
                    continue;
                }
                let mut d = k.clone();
                let verdict = if !decode_confident(&mut d).accepted {
                    "rejected"
                } else if d
                    .vertices
                    .iter()
                    .zip(&p.vertices)
                    .all(|(a, b)| (a.position - b.position).length() < 1e-3)
                {
                    "identical"
                } else {
                    "differs"
                };
                bump(format!("decoded {verdict}"));
            });
        }
    });
    for (k, n) in tally.into_inner().unwrap() {
        println!("{n:6}  {k}");
    }
}
