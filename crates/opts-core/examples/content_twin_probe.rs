//! How many scrambled (keyed) meshes have a plain copy of the same model anywhere in the
//! install, whatever its file name? A match means the exact geometry is available without any
//! decoding (`same_model`: same face list, same sorted coordinate triple at every vertex).
//!
//! Usage: `content_twin_probe <omsi root> [--list-names <substring>]`
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use opts_core::formats::o3d::parse;

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

fn model_hash(o: &opts_core::formats::o3d::O3d) -> u64 {
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
    for d in [
        "Vehicles",
        "Sceneryobjects",
        "Splines",
        "Humans",
        "Money",
        "TicketPacks",
    ] {
        walk(&root.join(d), &mut files);
    }
    eprintln!("{} files", files.len());
    let next = AtomicUsize::new(0);
    // (hash, keyed, lowercase file name, path)
    let rows: Mutex<Vec<(u64, bool, String, PathBuf)>> = Mutex::new(Vec::new());
    std::thread::scope(|s| {
        for _ in 0..14 {
            s.spawn(|| {
                let mut local = Vec::new();
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some(p) = files.get(i) else { break };
                    let Ok(b) = std::fs::read(p) else { continue };
                    let Ok(o) = parse(&b) else { continue };
                    if o.vertices.len() < 4 {
                        continue;
                    }
                    let name = p.file_name().unwrap().to_string_lossy().to_lowercase();
                    local.push((model_hash(&o), o.encryption_key.is_some(), name, p.clone()));
                }
                rows.lock().unwrap().extend(local);
            });
        }
    });
    let rows = rows.into_inner().unwrap();
    let mut plain: HashMap<u64, Vec<(String, PathBuf)>> = HashMap::new();
    for (h, keyed, n, p) in &rows {
        if !keyed {
            plain.entry(*h).or_default().push((n.clone(), p.clone()));
        }
    }
    let (mut keyed, mut same_name, mut any_name, mut only_keyed_dup) = (0, 0, 0, 0);
    let mut keyed_groups: HashMap<u64, usize> = HashMap::new();
    for (h, k, _, _) in &rows {
        if *k {
            *keyed_groups.entry(*h).or_default() += 1;
        }
    }
    let mut renamed: Vec<String> = Vec::new();
    let mut have_twin: HashSet<PathBuf> = HashSet::new();
    for (h, k, n, p) in &rows {
        if !*k {
            continue;
        }
        keyed += 1;
        match plain.get(h) {
            Some(c) if c.iter().any(|(m, _)| m == n) => {
                same_name += 1;
                any_name += 1;
                have_twin.insert(p.clone());
            }
            Some(c) => {
                any_name += 1;
                have_twin.insert(p.clone());
                renamed.push(format!("{} -> {}", p.display(), c[0].1.display()));
            }
            None => {
                if keyed_groups[h] > 1 {
                    only_keyed_dup += 1;
                }
            }
        }
    }
    println!("keyed meshes: {keyed}");
    println!("  plain twin with the same file name: {same_name}");
    println!(
        "  plain twin under any name:          {any_name} (+{} renamed)",
        any_name - same_name
    );
    println!("  no plain twin, but keyed copies:    {only_keyed_dup}");
    for r in renamed.iter().take(15) {
        println!("    {r}");
    }
    if let Ok(out) = std::env::var("COPIES_LIST") {
        // path <TAB> other keyed copies <TAB> plain copies, for every keyed mesh.
        let mut keyed_by_hash: HashMap<u64, usize> = HashMap::new();
        for (h, k, _, _) in &rows {
            if *k {
                *keyed_by_hash.entry(*h).or_default() += 1;
            }
        }
        let mut text = String::new();
        for (h, k, _, p) in &rows {
            if *k {
                text.push_str(&format!(
                    "{}\t{}\t{}\n",
                    p.display(),
                    keyed_by_hash[h] - 1,
                    plain.get(h).map_or(0, Vec::len)
                ));
            }
        }
        std::fs::write(out, text).unwrap();
    }
    let out = std::env::var("TWIN_LIST").ok();
    if let Some(out) = out {
        let mut s = String::new();
        for p in &have_twin {
            s.push_str(&format!("{}\n", p.display()));
        }
        std::fs::write(out, s).unwrap();
    }
}
