//! Evaluates `o3d::exact::decode` on every keyed `.o3d` with a plain copy of the same model:
//! how many meshes come out with exactly the plain copy's positions, texture coordinates and
//! normals. Files with key `0` are reported separately.
//!
//! Usage: `exact_twin_eval <omsi root> [--list]`
use std::collections::{BTreeMap, HashMap};
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use opts_core::formats::o3d::{exact, parse, O3d};

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
    let list = args.next().as_deref() == Some("--list");
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
    let tally: Mutex<BTreeMap<String, usize>> = Mutex::new(BTreeMap::new());
    let rows: Mutex<Vec<String>> = Mutex::new(Vec::new());
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
                let key0 = k.encryption_key == Some(0);
                let verdict = match exact::decode(&mut k) {
                    None => "no seed".to_string(),
                    Some(_) => {
                        let n = k.vertices.len();
                        let pos = k
                            .vertices
                            .iter()
                            .zip(&p.vertices)
                            .filter(|(a, b)| a.position == b.position)
                            .count();
                        let uv = k
                            .vertices
                            .iter()
                            .zip(&p.vertices)
                            .filter(|(a, b)| (a.uv - b.uv).abs().max_element() < 1e-5)
                            .count();
                        let nrm = k
                            .vertices
                            .iter()
                            .zip(&p.vertices)
                            .filter(|(a, b)| (a.normal - b.normal).abs().max_element() < 1e-4)
                            .count();
                        let share = |c: usize| {
                            if c == n {
                                "all"
                            } else if c * 100 >= n * 99 {
                                "≥99%"
                            } else if c * 100 >= n * 90 {
                                "≥90%"
                            } else {
                                "<90%"
                            }
                        };
                        format!(
                            "positions {:5} | uv {:5} | normals {:5}",
                            share(pos),
                            share(uv),
                            share(nrm)
                        )
                    }
                };
                let key = format!("{} | {verdict}", if key0 { "key 0   " } else { "keyed   " });
                *tally.lock().unwrap().entry(key).or_default() += 1;
                if list {
                    rows.lock().unwrap().push(format!(
                        "{}\t{verdict}",
                        kp.strip_prefix(&root).unwrap_or(kp).display()
                    ));
                }
            });
        }
    });
    let mut t: Vec<_> = tally.into_inner().unwrap().into_iter().collect();
    t.sort_by_key(|r| std::cmp::Reverse(r.1));
    for (k, n) in t {
        println!("{n:6}  {k}");
    }
    if list {
        for r in rows.into_inner().unwrap() {
            println!("{r}");
        }
    }
}
