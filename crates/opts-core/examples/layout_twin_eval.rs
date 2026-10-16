//! Which frame does a decoded keyed mesh come out in? Checks every keyed `.o3d` of the install
//! that has a plain copy of the same model (same face list, same sorted coordinate triple at
//! every vertex, as in `content_twin_probe`): the plain copy's stored normals are intact, so its
//! layout (`O3d::vertex_layout`) is the truth. The keyed copy is decoded and compared with the
//! plain one vertex by vertex, and the layout `O3d::vertex_layout` gives the decoded mesh is
//! compared with the truth.
//!
//! Usage: `layout_twin_eval <omsi root>` (decoder overrides from the environment, e.g. `W_NLAB=1`;
//! `LIST=<file>` writes one verdict per mesh).
use std::collections::{BTreeMap, HashMap};
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use opts_core::formats::o3d::descramble::{decode_confident_with, DecodeParams};
use opts_core::formats::o3d::{parse, O3d, VertexLayout};

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

/// The frame a layout puts "up" in: stored `x` for axis-cycled meshes, stored `y` otherwise.
fn up_axis(l: VertexLayout) -> &'static str {
    match l {
        VertexLayout::AxisCycled => "x-up",
        VertexLayout::Unreadable => "unreadable",
        _ => "y-up",
    }
}

/// The six axis permutations: `out[k] = in[P[k]]`.
const PERMS6: [[usize; 3]; 6] = [
    [0, 1, 2],
    [0, 2, 1],
    [1, 0, 2],
    [1, 2, 0],
    [2, 0, 1],
    [2, 1, 0],
];

fn perm(p: usize, v: glam::Vec3) -> glam::Vec3 {
    let a = [v.x, v.y, v.z];
    let q = PERMS6[p];
    glam::Vec3::new(a[q[0]], a[q[1]], a[q[2]])
}

/// Connected parts of the face graph (vertex index -> part).
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
        let (a, b, c) = (t[0] as usize, t[1] as usize, t[2] as usize);
        if a < n && b < n && c < n {
            let (ra, rb, rc) = (find(&mut up, a), find(&mut up, b), find(&mut up, c));
            up[rb] = ra;
            let rc2 = find(&mut up, rc);
            up[rc2] = ra;
        }
    }
    (0..n).map(|i| find(&mut up, i)).collect()
}

/// How the decoded positions relate to the plain copy's.
fn relation(k: &O3d, p: &O3d) -> String {
    let close = |a: glam::Vec3, b: glam::Vec3| (a - b).length() < 1e-3;
    let n = p.vertices.len();
    let same = (0..n)
        .filter(|&i| close(k.vertices[i].position, p.vertices[i].position))
        .count();
    if same == n {
        return "identical".into();
    }
    for (g, q) in PERMS6.iter().enumerate().skip(1) {
        let m = (0..n)
            .filter(|&i| close(k.vertices[i].position, perm(g, p.vertices[i].position)))
            .count();
        if m == n {
            return format!("global {q:?}");
        }
    }
    // Per part: every part matches the plain copy under one permutation?
    let part = parts(p);
    let mut by: HashMap<usize, Vec<usize>> = HashMap::new();
    for (i, r) in part.iter().enumerate() {
        by.entry(*r).or_default().push(i);
    }
    let mut perms_used = std::collections::BTreeSet::new();
    let mut unexplained = 0usize;
    for vs in by.values() {
        let hit = (0..6).find(|&g| {
            vs.iter()
                .all(|&i| close(k.vertices[i].position, perm(g, p.vertices[i].position)))
        });
        match hit {
            Some(g) => {
                perms_used.insert(g);
            }
            None => unexplained += vs.len(),
        }
    }
    if unexplained == 0 {
        format!(
            "per-part ({} parts, {} perms)",
            by.len().min(9),
            perms_used.len()
        )
    } else if unexplained * 100 <= n * 5 {
        format!("per-part + ≤5% stray vertices ({} perms)", perms_used.len())
    } else {
        format!("other (equal vertices {}0 %)", same * 10 / n)
    }
}

fn main() {
    let root = PathBuf::from(std::env::args().nth(1).expect("omsi root"));
    let params = DecodeParams::default().with_overrides(|k| std::env::var(k).ok());
    // `LIST=<file>`: one line per keyed mesh with its verdict (to compare two parameter sets).
    let list = std::env::var("LIST").ok();
    let rows: Mutex<Vec<String>> = Mutex::new(Vec::new());
    let mut files = Vec::new();
    walk(&root, &mut files);
    // Plain copies by model, then every keyed file whose model has one.
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
    eprintln!(
        "{} files, {} keyed meshes with a plain copy",
        files.len(),
        pairs.len()
    );

    // (truth frame, decoded frame, decoded floor axes, positions identical) -> count, examples
    let tally: Mutex<BTreeMap<String, (usize, Vec<String>)>> = Mutex::new(BTreeMap::new());
    let next = AtomicUsize::new(0);
    std::thread::scope(|s| {
        for _ in 0..threads {
            s.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::Relaxed);
                let Some((kp, pp)) = pairs.get(i) else { break };
                let (Ok(kb), Ok(pb)) = (std::fs::read(kp), std::fs::read(pp)) else {
                    continue;
                };
                let (Ok(mut k), Ok(p)) = (parse(&kb), parse(&pb)) else {
                    continue;
                };
                let truth = up_axis(p.vertex_layout());
                let key = if decode_confident_with(&mut k, &params).accepted {
                    format!(
                        "truth {truth:5} plain-floor {:9} | decoded floor {:9} | {}",
                        format!("{:?}", p.floor_axes()),
                        format!("{:?}", k.floor_axes()),
                        relation(&k, &p),
                    )
                } else {
                    format!("truth {truth:5} | rejected")
                };
                if list.is_some() {
                    rows.lock().unwrap().push(format!(
                        "{}\t{key}",
                        kp.strip_prefix(&root).unwrap_or(kp).display()
                    ));
                }
                let mut t = tally.lock().unwrap();
                let e = t.entry(key).or_default();
                e.0 += 1;
                if e.1.len() < 3 {
                    e.1.push(kp.strip_prefix(&root).unwrap_or(kp).display().to_string());
                }
            });
        }
    });
    if let Some(path) = &list {
        let mut r = rows.into_inner().unwrap();
        r.sort();
        std::fs::write(path, r.join("\n")).expect("write LIST");
    }
    let mut rows: Vec<_> = tally.into_inner().unwrap().into_iter().collect();
    rows.sort_by_key(|r| std::cmp::Reverse(r.1 .0));
    for (k, (n, ex)) in rows {
        println!("{n:6}  {k}   e.g. {ex:?}");
    }
}
