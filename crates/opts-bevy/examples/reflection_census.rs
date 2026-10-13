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

const TEXT_EXT: [&str; 8] = ["cfg", "sco", "sli", "bus", "ovh", "osc", "xml", "txt"];
const GLASS_WORDS: [&str; 9] = [
    "glas", "glass", "scheibe", "fenster", "window", "verglas", "windscreen", "windshield", "pane",
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

fn main() {
    let Some(root) = std::env::args().nth(1) else {
        eprintln!("usage: reflection_census <omsi-root>");
        return;
    };
    let root = PathBuf::from(root);
    let mut files = Vec::new();
    walk(&root, &mut files);
    files.sort();

    let mut kw: BTreeMap<String, Tally> = BTreeMap::new();
    let mut alpha_vals: BTreeMap<String, u64> = BTreeMap::new();
    let mut envmap_tex: BTreeMap<String, u64> = BTreeMap::new();
    let mut envmap_factor: BTreeMap<String, u64> = BTreeMap::new();
    let mut glass_alpha: BTreeMap<String, u64> = BTreeMap::new();
    let mut glass_tex: BTreeMap<String, u64> = BTreeMap::new();
    let mut mirror_tex: BTreeMap<String, u64> = BTreeMap::new();
    let mut mirror_env: u64 = 0;
    let mut glass_env: u64 = 0;
    let mut mirror_mats: u64 = 0;
    let mut glass_mats: u64 = 0;
    let mut total_matl: u64 = 0;
    let mut total_matl_files: BTreeSet<String> = BTreeSet::new();
    let mut env_nonglass_mats: u64 = 0;
    let mut name_hits: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    let mut text_files = 0u64;
    let mut vehicles: BTreeSet<String> = BTreeSet::new();
    let mut objects: BTreeSet<String> = BTreeSet::new();
    let mut sco_files = 0u64;
    let mut env_files_bmp: BTreeMap<String, u64> = BTreeMap::new();

    for f in &files {
        let rel = f
            .strip_prefix(&root)
            .unwrap_or(f)
            .to_string_lossy()
            .replace('\\', "/");
        let lower_rel = rel.to_lowercase();
        let ext = f
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        let unit = unit_of(&rel);
        if unit.starts_with("V:") {
            vehicles.insert(unit.clone());
        }
        if unit.starts_with("O:") {
            objects.insert(unit.clone());
        }
        if ext == "sco" {
            sco_files += 1;
        }
        // File-name hits (any type).
        let fname = lower_rel.rsplit('/').next().unwrap_or("").to_string();
        if has_any(&fname, &MIRROR_WORDS) {
            let key = if matches!(ext.as_str(), "o3d" | "x" | "bmp" | "dds" | "tga" | "jpg" | "png") {
                "mirror-named file"
            } else {
                "mirror-named other"
            };
            name_hits.entry(key).or_default().push(rel.clone());
        }
        if fname.contains("envmap") || fname.contains("env_map") || fname.contains("reflect") {
            *env_files_bmp.entry(fname.clone()).or_default() += 1;
        }
        if !TEXT_EXT.contains(&ext.as_str()) {
            continue;
        }
        let Ok(bytes) = std::fs::read(f) else {
            continue;
        };
        text_files += 1;
        let text: String = bytes.iter().map(|&b| b as char).collect();
        let lines: Vec<String> = text.lines().map(|l| l.trim().to_lowercase()).collect();
        let mut cur_tex = String::new();
        let mut cur_has_env = false;
        let mut cur_alpha = String::from("-");
        let mut in_mat = false;
        let flush = |in_mat: bool,
                         tex: &str,
                         has_env: bool,
                         alpha: &str,
                         glass_alpha: &mut BTreeMap<String, u64>,
                         glass_tex: &mut BTreeMap<String, u64>,
                         mirror_tex: &mut BTreeMap<String, u64>,
                         glass_mats: &mut u64,
                         mirror_mats: &mut u64,
                         glass_env: &mut u64,
                         mirror_env: &mut u64,
                         env_nonglass: &mut u64| {
            if !in_mat {
                return;
            }
            let t = tex.rsplit(['/', '\\']).next().unwrap_or(tex);
            let g = has_any(t, &GLASS_WORDS);
            let m = has_any(t, &MIRROR_WORDS);
            if g {
                *glass_mats += 1;
                *glass_alpha.entry(alpha.to_string()).or_default() += 1;
                *glass_tex.entry(t.to_string()).or_default() += 1;
                if has_env {
                    *glass_env += 1;
                }
            }
            if m {
                *mirror_mats += 1;
                *mirror_tex.entry(t.to_string()).or_default() += 1;
                if has_env {
                    *mirror_env += 1;
                }
            }
            if has_env && !g && !m {
                *env_nonglass += 1;
            }
        };
        let mut i = 0;
        while i < lines.len() {
            let l = &lines[i];
            if l.starts_with('[') && l.ends_with(']') {
                let name = &l[1..l.len() - 1];
                let next = lines.get(i + 1).map(String::as_str).unwrap_or("");
                if name == "matl" {
                    flush(
                        in_mat,
                        &cur_tex,
                        cur_has_env,
                        &cur_alpha,
                        &mut glass_alpha,
                        &mut glass_tex,
                        &mut mirror_tex,
                        &mut glass_mats,
                        &mut mirror_mats,
                        &mut glass_env,
                        &mut mirror_env,
                        &mut env_nonglass_mats,
                    );
                    in_mat = true;
                    cur_tex = next.to_string();
                    cur_has_env = false;
                    cur_alpha = "-".into();
                    total_matl += 1;
                    total_matl_files.insert(rel.clone());
                }
                let interesting = matches!(
                    name,
                    "matl_envmap"
                        | "matl_envmap_mask"
                        | "matl_alpha"
                        | "matl_transmap"
                        | "matl_bumpmap"
                        | "matl_nightmap"
                        | "matl_lightmap"
                        | "matl_noZwrite"
                        | "matl_nozwrite"
                        | "matl_nozcheck"
                ) || has_any(name, &["mirror", "spiegel", "reflect", "cube", "env"]);
                if interesting {
                    kw.entry(name.to_string()).or_default().add(&rel, &unit);
                }
                if name == "matl_alpha" {
                    *alpha_vals.entry(next.to_string()).or_default() += 1;
                    cur_alpha = next.to_string();
                }
                if name == "matl_envmap" {
                    cur_has_env = true;
                    *envmap_tex.entry(next.to_string()).or_default() += 1;
                    let fac = lines.get(i + 2).map(String::as_str).unwrap_or("");
                    *envmap_factor.entry(fac.to_string()).or_default() += 1;
                }
            } else if ext == "osc" || ext == "txt" || ext == "cfg" {
                // Script or free text mentioning mirrors.
                if (l.contains("mirror") || l.contains("spiegel")) && !l.starts_with("//") && !l.starts_with('#') {
                    name_hits.entry("text line mentions mirror").or_default().push(format!("{rel}: {}", l.chars().take(80).collect::<String>()));
                }
            }
            i += 1;
        }
        flush(
            in_mat,
            &cur_tex,
            cur_has_env,
            &cur_alpha,
            &mut glass_alpha,
            &mut glass_tex,
            &mut mirror_tex,
            &mut glass_mats,
            &mut mirror_mats,
            &mut glass_env,
            &mut mirror_env,
            &mut env_nonglass_mats,
        );
    }

    println!("files walked: {}  text files read: {text_files}", files.len());
    println!(
        "vehicle folders: {}  object folders: {}  .sco files: {sco_files}",
        vehicles.len(),
        objects.len()
    );
    println!(
        "materials ([matl]): {total_matl} in {} files",
        total_matl_files.len()
    );
    println!("\n== keyword | uses | files | vehicle/object units");
    for (k, t) in &kw {
        let v = t.units.iter().filter(|u| u.starts_with("V:")).count();
        let o = t.units.iter().filter(|u| u.starts_with("O:")).count();
        println!(
            "[{k}] | {} | {} | {v} vehicle dirs, {o} object dirs, {} other",
            t.count,
            t.files.len(),
            t.units.len() - v - o
        );
    }
    println!("\n== [matl_alpha] values (all)");
    for (k, v) in &alpha_vals {
        println!("{k:?}: {v}");
    }
    println!("\n== [matl_envmap] texture (top 15)");
    top(&envmap_tex, 15);
    println!("\n== [matl_envmap] factor (top 15)");
    top(&envmap_factor, 15);
    println!("\n== glass-named materials: {glass_mats} (with envmap {glass_env})");
    println!("alpha mode of glass-named materials:");
    top(&glass_alpha, 10);
    println!("glass texture names (top 15):");
    top(&glass_tex, 15);
    println!(
        "\n== mirror-named materials: {mirror_mats} (with envmap {mirror_env})"
    );
    top(&mirror_tex, 15);
    println!("\n== env-mapped materials that are neither glass- nor mirror-named: {env_nonglass_mats}");
    println!("\n== envmap/reflect-named files (top 10)");
    top(&env_files_bmp, 10);
    println!("\n== name hits");
    for (k, v) in &name_hits {
        let mut u: BTreeSet<&str> = BTreeSet::new();
        for s in v {
            u.insert(s);
        }
        println!("{k}: {} hits", v.len());
        for s in u.iter().take(12) {
            println!("   {s}");
        }
    }
}

fn top(m: &BTreeMap<String, u64>, n: usize) {
    let mut v: Vec<_> = m.iter().collect();
    v.sort_by_key(|(_, c)| std::cmp::Reverse(**c));
    for (k, c) in v.into_iter().take(n) {
        println!("   {c:>6}  {k}");
    }
}
