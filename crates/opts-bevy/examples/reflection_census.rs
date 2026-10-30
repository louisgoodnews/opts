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

/// The `[matl]` block being read.
struct Mat {
    in_mat: bool,
    tex: String,
    has_env: bool,
    alpha: String,
}

impl Mat {
    fn new() -> Self {
        Self {
            in_mat: false,
            tex: String::new(),
            has_env: false,
            alpha: String::from("-"),
        }
    }
}

impl Census {
    /// Classifies the finished material by its texture name.
    fn flush(&mut self, m: &Mat) {
        if !m.in_mat {
            return;
        }
        let t = m.tex.rsplit(['/', '\\']).next().unwrap_or(&m.tex);
        let g = has_any(t, &GLASS_WORDS);
        let mir = has_any(t, &MIRROR_WORDS);
        if g {
            self.glass_mats += 1;
            *self.glass_alpha.entry(m.alpha.to_string()).or_default() += 1;
            *self.glass_tex.entry(t.to_string()).or_default() += 1;
            if m.has_env {
                self.glass_env += 1;
            }
        }
        if mir {
            self.mirror_mats += 1;
            *self.mirror_tex.entry(t.to_string()).or_default() += 1;
            if m.has_env {
                self.mirror_env += 1;
            }
        }
        if m.has_env && !g && !mir {
            self.env_nonglass_mats += 1;
        }
    }

    /// File-name statistics (any file type); `fname` is the lower-case base name.
    fn scan_name(&mut self, rel: &str, ext: &str, fname: &str) {
        if has_any(fname, &MIRROR_WORDS) {
            let key = if matches!(ext, "o3d" | "x" | "bmp" | "dds" | "tga" | "jpg" | "png") {
                "mirror-named file"
            } else {
                "mirror-named other"
            };
            self.name_hits.entry(key).or_default().push(rel.to_string());
        }
        if fname.contains("envmap") || fname.contains("env_map") || fname.contains("reflect") {
            *self.env_files_bmp.entry(fname.to_string()).or_default() += 1;
        }
    }

    /// One `[keyword]` line (`lines[i]`) of a text file; `at` is (relative path, unit).
    fn on_header(&mut self, m: &mut Mat, lines: &[String], i: usize, at: (&str, &str)) {
        let (rel, unit) = at;
        let l = &lines[i];
        let name = &l[1..l.len() - 1];
        let next = lines.get(i + 1).map(String::as_str).unwrap_or("");
        if name == "matl" {
            self.flush(m);
            *m = Mat {
                in_mat: true,
                tex: next.to_string(),
                has_env: false,
                alpha: "-".into(),
            };
            self.total_matl += 1;
            self.total_matl_files.insert(rel.to_string());
        }
        let interesting =
            matches!(
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
            self.kw.entry(name.to_string()).or_default().add(rel, unit);
        }
        if name == "matl_alpha" {
            *self.alpha_vals.entry(next.to_string()).or_default() += 1;
            m.alpha = next.to_string();
        }
        if name == "matl_envmap" {
            m.has_env = true;
            *self.envmap_tex.entry(next.to_string()).or_default() += 1;
            let fac = lines.get(i + 2).map(String::as_str).unwrap_or("");
            *self.envmap_factor.entry(fac.to_string()).or_default() += 1;
        }
    }

    /// Scans the trimmed, lower-cased lines of one text file.
    fn scan_lines(&mut self, lines: &[String], ext: &str, at: (&str, &str)) {
        let mut m = Mat::new();
        for (i, l) in lines.iter().enumerate() {
            if l.starts_with('[') && l.ends_with(']') {
                self.on_header(&mut m, lines, i, at);
            } else if ext == "osc" || ext == "txt" || ext == "cfg" {
                // Script or free text mentioning mirrors.
                if (l.contains("mirror") || l.contains("spiegel"))
                    && !l.starts_with("//")
                    && !l.starts_with('#')
                {
                    self.name_hits
                        .entry("text line mentions mirror")
                        .or_default()
                        .push(format!("{}: {}", at.0, l.chars().take(80).collect::<String>()));
                }
            }
        }
        self.flush(&m);
    }

    fn visit(&mut self, f: &Path, root: &Path) {
        let rel = f
            .strip_prefix(root)
            .unwrap_or(f)
            .to_string_lossy()
            .replace('\\', "/");
        let ext = f
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        let unit = unit_of(&rel);
        if unit.starts_with("V:") {
            self.vehicles.insert(unit.clone());
        }
        if unit.starts_with("O:") {
            self.objects.insert(unit.clone());
        }
        if ext == "sco" {
            self.sco_files += 1;
        }
        let lower_rel = rel.to_lowercase();
        self.scan_name(&rel, &ext, lower_rel.rsplit('/').next().unwrap_or(""));
        if !TEXT_EXT.contains(&ext.as_str()) {
            return;
        }
        let Ok(bytes) = std::fs::read(f) else {
            return;
        };
        self.text_files += 1;
        let text: String = bytes.iter().map(|&b| b as char).collect();
        let lines: Vec<String> = text.lines().map(|l| l.trim().to_lowercase()).collect();
        self.scan_lines(&lines, &ext, (&rel, &unit));
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
    let mut census = Census::default();
    for f in &files {
        census.visit(f, &root);
    }
    census.print_report(files.len());
}
