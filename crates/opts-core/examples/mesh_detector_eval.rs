#![allow(clippy::type_complexity)]
//! Precision/recall of the mesh plausibility detectors (`o3d::audit::check`) on decoded keyed
//! meshes, measured against their plain twins (ground truth) (P-1014).
//!
//! A decoded mesh is *wrong* when fewer than 98 % of its vertices are within 1 % of the bounding
//! diagonal of the twin's. Usage: `mesh_detector_eval <omsi root> [--threads N]`.
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use opts_core::formats::o3d::audit::check;
use opts_core::formats::o3d::descramble::decode_confident;
use opts_core::formats::o3d::parse;

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        let p = e.path();
        match e.file_type() {
            Ok(t) if t.is_dir() => walk(&p, out),
            _ if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("o3d")) => out.push(p),
            _ => {}
        }
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let root = PathBuf::from(args.next().expect("omsi root"));
    let mut threads = 8usize;
    while let Some(a) = args.next() {
        if a == "--threads" {
            threads = args.next().and_then(|s| s.parse().ok()).unwrap_or(8);
        }
    }
    let mut files = Vec::new();
    walk(&root, &mut files);
    files.sort();
    let mut by: HashMap<(String, usize, usize), (Option<PathBuf>, Option<PathBuf>)> =
        HashMap::new();
    for p in &files {
        let Ok(b) = std::fs::read(p) else { continue };
        if b.len() < 8 || b[0] != 0x84 || b[1] != 0x19 || b[2] < 4 {
            continue;
        }
        let Ok(o) = parse(&b) else { continue };
        let e = by
            .entry((
                p.file_name().unwrap().to_string_lossy().to_lowercase(),
                o.vertices.len(),
                o.indices.len(),
            ))
            .or_default();
        if o.encryption_key.is_some() {
            e.1.get_or_insert(p.clone());
        } else {
            e.0.get_or_insert(p.clone());
        }
    }
    let pairs: Vec<(PathBuf, PathBuf)> = by.into_values().filter_map(|(a, b)| a.zip(b)).collect();
    eprintln!("{} candidate pairs", pairs.len());
    let next = AtomicUsize::new(0);
    // per code: (wrong flagged, right flagged); totals
    let tally: Mutex<BTreeMap<String, (usize, usize)>> = Mutex::new(BTreeMap::new());
    let totals = Mutex::new((0usize, 0usize, 0usize)); // pairs, accepted, wrong
    let listing = Mutex::new(Vec::new());
    let rule = Mutex::new([(0usize, 0usize); 6]);
    std::thread::scope(|s| {
        for _ in 0..threads {
            s.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::Relaxed);
                let Some((pp, ep)) = pairs.get(i) else { break };
                let (Ok(po), Ok(mut eo)) = (
                    parse(&std::fs::read(pp).unwrap()),
                    parse(&std::fs::read(ep).unwrap()),
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
                        let mut x = a.position.to_array();
                        let mut y = b.position.to_array();
                        x.sort_by(|a, b| a.total_cmp(b));
                        y.sort_by(|a, b| a.total_cmp(b));
                        (0..3).all(|k| (x[k] - y[k]).abs() < 1e-4)
                    })
                    .count();
                if (same as f32) < 0.95 * po.vertices.len() as f32 {
                    continue;
                }
                let rep = decode_confident(&mut eo);
                let mut t = totals.lock().unwrap();
                t.0 += 1;
                if !rep.accepted {
                    continue;
                }
                t.1 += 1;
                let (mut lo, mut hi) = (glam::Vec3::splat(f32::MAX), glam::Vec3::splat(f32::MIN));
                for v in &po.vertices {
                    lo = lo.min(v.position);
                    hi = hi.max(v.position);
                }
                let tol = 0.01 * (hi - lo).length();
                let ok = eo
                    .vertices
                    .iter()
                    .zip(&po.vertices)
                    .filter(|(a, b)| (a.position - b.position).length() <= tol)
                    .count() as f32
                    / po.vertices.len() as f32;
                let wrong = ok < 0.98;
                {
                    // long-edge rule evaluation: faces dropped, split by truth
                    let mut edges: Vec<f32> = Vec::new();
                    let tri_edge = |t: &[u32; 3]| {
                        let p = t.map(|k| eo.vertices[k as usize].position);
                        (p[1] - p[0])
                            .length()
                            .max((p[2] - p[0]).length())
                            .max((p[2] - p[1]).length())
                    };
                    for t in &eo.indices {
                        edges.push(tri_edge(t));
                    }
                    let mut sorted = edges.clone();
                    sorted.sort_by(|a, b| a.total_cmp(b));
                    let med = sorted[sorted.len() / 2];
                    let mut r = rule.lock().unwrap();
                    for (ki, k) in [6.0f32, 10.0, 15.0, 25.0, 40.0].iter().enumerate() {
                        for (ti, t) in eo.indices.iter().enumerate() {
                            if edges[ti] > med * k && edges[ti] > 1.0 {
                                let bad = t.iter().any(|&v| {
                                    (eo.vertices[v as usize].position
                                        - po.vertices[v as usize].position)
                                        .length()
                                        > tol
                                });
                                if bad {
                                    r[ki].0 += 1
                                } else {
                                    r[ki].1 += 1
                                }
                            }
                        }
                    }
                    r[5].0 += eo.indices.len();
                }
                if wrong {
                    t.2 += 1;
                }
                drop(t);
                let (mut codes, mut m) = (Vec::new(), Vec::new());
                check(&eo, "", &mut codes, &mut m);
                // codes the plain twin itself triggers are content, not decoding errors
                let (mut pc, mut pm) = (Vec::new(), Vec::new());
                check(&po, "", &mut pc, &mut pm);
                codes.retain(|c| !pc.contains(c));
                let mut tl = tally.lock().unwrap();
                for c in &codes {
                    let e = tl.entry(c.clone()).or_default();
                    if wrong {
                        e.0 += 1;
                    } else {
                        e.1 += 1;
                    }
                }
                let e = tl.entry("ANY".into()).or_default();
                if !codes.is_empty() {
                    if wrong {
                        e.0 += 1;
                    } else {
                        e.1 += 1;
                    }
                }
                if wrong {
                    listing.lock().unwrap().push(format!(
                        "{}\t{ok:.2}\t{}",
                        ep.display(),
                        codes.join(",")
                    ));
                }
            });
        }
    });
    let t = totals.into_inner().unwrap();
    let r = rule.into_inner().unwrap();
    for (ki, k) in [6, 10, 15, 25, 40].iter().enumerate() {
        println!(
            "long-edge rule > {k}x median: dropped bad {} good {} (of {} faces)",
            r[ki].0, r[ki].1, r[5].0
        );
    }
    println!(
        "twin pairs {} accepted {} wrong(<98% positions) {}",
        t.0, t.1, t.2
    );
    println!("code\tflagged_wrong\tflagged_right\trecall\tprecision");
    for (c, (w, r)) in tally.into_inner().unwrap() {
        println!(
            "{c}\t{w}\t{r}\t{:.2}\t{:.2}",
            w as f32 / t.2.max(1) as f32,
            w as f32 / (w + r).max(1) as f32
        );
    }
    let mut l = listing.into_inner().unwrap();
    l.sort();
    for x in l {
        eprintln!("WRONG\t{x}");
    }
}
