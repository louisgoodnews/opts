//! Developer tool (M-0085): corpus statistics of mirror, glass and environment-map materials.
//!
//! `cargo run --release -p opts-bevy --example reflection_census -- <omsi-root>`
//!
//! Reads text files only (`.cfg`, `.sco`, `.sli`, `.bus`, `.ovh`, `.osc`, `.xml`, `.txt`) and file
//! names; nothing is written, no binary is parsed. Counts, per keyword:
//!
//! * `[matl_envmap]`, `[matl_envmap_mask]`, `[matl_alpha]` values, `[matl_transmap]`,
//!   `[matl_bumpmap]` (files, materials, vehicles/objects that use them),
//! * every `[keyword]` whose name contains mirror / spiegel / reflect / cube / env,
//! * materials whose texture file name looks like glass or a mirror and what alpha mode and
//!   env-map they combine with,
//! * script and file names that talk about mirrors.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

#[path = "reflection_census/report.rs"]
mod report;
#[path = "reflection_census/scan.rs"]
mod scan;
#[path = "reflection_census/visit.rs"]
mod visit;

const TEXT_EXT: [&str; 8] = ["cfg", "sco", "sli", "bus", "ovh", "osc", "xml", "txt"];
const GLASS_WORDS: [&str; 9] = [
    "glas",
    "glass",
    "scheibe",
    "fenster",
    "window",
    "verglas",
    "windscreen",
    "windshield",
    "pane",
];
const MIRROR_WORDS: [&str; 5] = ["spiegel", "mirror", "mirr", "rueckspiegel", "rückspiegel"];

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let p = e.path();
        if p.is_dir() {
            walk(&p, out);
        } else {
            out.push(p);
        }
    }
}

fn has_any(s: &str, words: &[&str]) -> bool {
    words.iter().any(|w| s.contains(w))
}

#[derive(Default)]
struct Tally {
    files: BTreeSet<String>,
    units: BTreeSet<String>,
    count: u64,
}

impl Tally {
    fn add(&mut self, file: &str, unit: &str) {
        self.files.insert(file.to_string());
        self.units.insert(unit.to_string());
        self.count += 1;
    }
}

/// `Vehicles/<dir>/...` is a vehicle unit, `Sceneryobjects/<dir>/..` an object folder; other files
/// are their own unit.
fn unit_of(rel: &str) -> String {
    let parts: Vec<&str> = rel.split('/').collect();
    if parts.len() > 2 && parts[0].eq_ignore_ascii_case("vehicles") {
        format!("V:{}", parts[1])
    } else if parts.len() > 2 && parts[0].eq_ignore_ascii_case("sceneryobjects") {
        format!("O:{}", parts[1])
    } else {
        format!("X:{rel}")
    }
}

type Counts = BTreeMap<String, u64>;

#[derive(Default)]
struct Census {
    kw: BTreeMap<String, Tally>,
    alpha_vals: Counts,
    envmap_tex: Counts,
    envmap_factor: Counts,
    glass_alpha: Counts,
    glass_tex: Counts,
    mirror_tex: Counts,
    mirror_env: u64,
    glass_env: u64,
    mirror_mats: u64,
    glass_mats: u64,
    total_matl: u64,
    total_matl_files: BTreeSet<String>,
    env_nonglass_mats: u64,
    name_hits: BTreeMap<&'static str, Vec<String>>,
    text_files: u64,
    vehicles: BTreeSet<String>,
    objects: BTreeSet<String>,
    sco_files: u64,
    env_files_bmp: Counts,
}

fn main() {
    let Some(root) = std::env::args().nth(1) else {
        eprintln!("usage: reflection_census <omsi-root>");
        return;
    };
    let root = PathBuf::from(root);
    let mut files = Vec::new();
    walk(&root, &mut files);
    files.sort();
    let mut census = Census::default();
    for f in &files {
        census.visit(f, &root);
    }
    census.print_report(files.len());
}
