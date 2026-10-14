//! Census of `[couple_back]` consists: chain lengths (and loops), what the second line of
//! `[couple_back]` correlates with, and how heavy a trailer is against the unit in front of it.
//!
//! The mass ratio matters because the vehicle simulation only knows the head unit's `[mass]`
//! (`docs/src/architecture/articulation-physics.md`, P2): a trailer that weighs 0.9 of its head
//! makes the consist twice as heavy as it is simulated.
//!
//! Usage: `consist_census <omsi root> [--pairs]` (`--pairs` prints one TSV row per head/trailer
//! pair with both masses: head, trailer, head t, trailer t, reversed).
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use opts_core::formats::vehicles::bus::BusManifest;

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            walk(&p, out);
        } else if p
            .extension()
            .is_some_and(|x| x.eq_ignore_ascii_case("bus") || x.eq_ignore_ascii_case("ovh"))
        {
            out.push(p);
        }
    }
}

/// Lexically normalised, lower-cased path key (`..` and `.` resolved, either separator).
fn key(path: &Path, rel: &str) -> String {
    let mut parts: Vec<String> = path
        .components()
        .map(|c| c.as_os_str().to_string_lossy().to_lowercase())
        .collect();
    for comp in rel.split(['\\', '/']) {
        match comp.trim() {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            c => parts.push(c.to_lowercase()),
        }
    }
    parts.join("/")
}

fn percentile(sorted: &[f32], p: f32) -> f32 {
    sorted[((sorted.len() - 1) as f32 * p).round() as usize]
}

fn main() {
    let mut args = std::env::args().skip(1);
    let root = PathBuf::from(args.next().expect("omsi root"));
    let pairs = args.next().as_deref() == Some("--pairs");
    let mut files = Vec::new();
    walk(&root.join("Vehicles"), &mut files);
    let by_key: HashMap<String, usize> = files
        .iter()
        .enumerate()
        .map(|(i, p)| (key(p, ""), i))
        .collect();
    let manifests: Vec<Option<BusManifest>> = files
        .iter()
        .map(|p| {
            std::fs::read(p)
                .ok()
                .and_then(|b| BusManifest::parse_bytes(&b).ok())
        })
        .collect();
    let target = |i: usize| -> Option<usize> {
        let name = manifests[i].as_ref()?.couple_back.as_ref()?;
        let dir = files[i].parent()?;
        by_key.get(&key(dir, name)).copied()
    };
    let with_link: Vec<usize> = (0..files.len())
        .filter(|&i| {
            manifests[i]
                .as_ref()
                .is_some_and(|m| m.couple_back.is_some())
        })
        .collect();
    // The same count on the raw text, tolerating indentation: `scan_blocks` only accepts a keyword
    // at the start of a line, so an indented `[couple_back]` is read as a parameter line.
    let raw_with_link = files
        .iter()
        .filter(|p| {
            std::fs::read(p).is_ok_and(|b| {
                String::from_utf8_lossy(&b)
                    .lines()
                    .any(|l| l.trim().eq_ignore_ascii_case("[couple_back]"))
            })
        })
        .count();
    println!(
        "{} vehicle files, {} parse, {} with [couple_back] as OPTS reads them ({} on the raw text, indentation tolerated)",
        files.len(),
        manifests.iter().flatten().count(),
        with_link.len(),
        raw_with_link
    );

    // The second line against "the target has a [couple_back] itself".
    let mut flag: HashMap<(bool, &str), usize> = HashMap::new();
    for &i in &with_link {
        let m = manifests[i].as_ref().unwrap();
        let what = match target(i) {
            None => "target not found",
            Some(t)
                if manifests[t]
                    .as_ref()
                    .is_some_and(|x| x.couple_back.is_some()) =>
            {
                "target has its own [couple_back]"
            }
            Some(_) => "target is the last unit",
        };
        *flag.entry((m.couple_back_reversed, what)).or_default() += 1;
    }
    println!("second line of [couple_back] (true = OPTS: coupled facing backwards):");
    let mut rows: Vec<_> = flag.into_iter().collect();
    rows.sort();
    for ((rev, what), n) in rows {
        println!("  {rev:5}  {what:36} {n}");
    }

    // Chains from heads (files no other file names as its trailer).
    let named: HashSet<usize> = with_link.iter().filter_map(|&i| target(i)).collect();
    let mut shape: HashMap<String, usize> = HashMap::new();
    let mut loops = Vec::new();
    for &h in with_link.iter().filter(|h| !named.contains(h)) {
        let (mut seen, mut cur) = (vec![h], h);
        let verdict = loop {
            match target(cur) {
                None if manifests[cur]
                    .as_ref()
                    .is_some_and(|m| m.couple_back.is_some()) =>
                {
                    break format!(
                        "{} behind the head, then an unresolved name",
                        seen.len() - 1
                    )
                }
                None => break format!("{} behind the head", seen.len() - 1),
                Some(t) if seen.contains(&t) => {
                    loops.push(files[h].display().to_string());
                    break "loops back on itself".to_string();
                }
                Some(t) => {
                    seen.push(t);
                    cur = t;
                }
            }
        };
        *shape.entry(verdict).or_default() += 1;
    }
    println!(
        "chains from the {} head files:",
        shape.values().sum::<usize>()
    );
    let mut rows: Vec<_> = shape.into_iter().collect();
    rows.sort();
    for (what, n) in rows {
        println!("  {what:50} {n}");
    }
    for l in loops.iter().take(15) {
        println!("    loop: {l}");
    }

    // Trailer mass against the unit in front of it.
    let mut ratios = Vec::new();
    let (mut no_mass, mut both) = (0, 0);
    for &i in &with_link {
        let Some(t) = target(i) else { continue };
        let (a, b) = (
            manifests[i].as_ref().and_then(|m| m.mass_tonnes),
            manifests[t].as_ref().and_then(|m| m.mass_tonnes),
        );
        match (a, b) {
            (Some(a), Some(b)) if a > 0.0 && b > 0.0 => {
                both += 1;
                ratios.push(b / a);
                if pairs {
                    println!(
                        "PAIR\t{}\t{}\t{a}\t{b}\t{}",
                        files[i].display(),
                        files[t].display(),
                        manifests[i].as_ref().unwrap().couple_back_reversed
                    );
                }
            }
            _ => no_mass += 1,
        }
    }
    ratios.sort_by(f32::total_cmp);
    println!("trailer mass / mass of the unit in front: {both} links with both masses, {no_mass} without");
    if !ratios.is_empty() {
        println!(
            "  p10 {:.2}  median {:.2}  p90 {:.2}  (simulated mass is the head's only: a consist of head + one trailer weighs 1 + ratio times that)",
            percentile(&ratios, 0.1),
            percentile(&ratios, 0.5),
            percentile(&ratios, 0.9)
        );
    }
}
