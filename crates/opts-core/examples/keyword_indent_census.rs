//! How much does the shared block scanner (`formats::text::scan_blocks`) drop because it accepts a
//! keyword only at the very start of a line?
//!
//! For every text file of the given kinds the raw keyword lines (a line that is, after trimming,
//! exactly `[name]`) are counted next to the blocks the scanner returns; the difference is what
//! OPTS silently reads as a parameter of the previous block. Output per extension: files, files with
//! an indented keyword, keyword lines lost, and the most frequent lost keywords. Also reported:
//! indented bracket lines whose name never occurs at the start of a line anywhere in the same
//! extension (probably author comments, not keywords), and files whose keyword occurs only indented.
//!
//! Last section: do the files that an indented keyword names exist, compared with a keyword at the
//! start of a line? (Disabled blocks name files that are not there.)
//!
//! Usage: `keyword_indent_census <omsi root> [ext,ext,...]`
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

use opts_core::formats::text::{decode_bytes, scan_blocks};

fn walk(dir: &Path, exts: &[String], out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            walk(&p, exts, out);
        } else if p
            .extension()
            .is_some_and(|x| exts.iter().any(|e| x.eq_ignore_ascii_case(e)))
        {
            out.push(p);
        }
    }
}

/// `[name]` alone on the (trimmed) line, name without brackets.
fn raw_keyword(line: &str) -> Option<String> {
    let t = line.trim();
    let inner = t.strip_prefix('[')?.strip_suffix(']')?;
    (!inner.is_empty() && !inner.contains(['[', ']'])).then(|| inner.to_ascii_lowercase())
}

#[derive(Default)]
struct Stats {
    files: usize,
    files_indented: usize,
    raw_keywords: usize,
    parsed_keywords: usize,
    lost: BTreeMap<String, usize>,
    /// Indented names that occur at column 0 nowhere in the extension.
    foreign: BTreeMap<String, usize>,
    /// Files in which a keyword occurs, but only indented.
    only_indented: BTreeMap<String, usize>,
}

/// Case-insensitive existence of `rel` (either separator) below `base`.
fn exists_ci(base: &Path, rel: &str) -> bool {
    let mut cur = base.to_path_buf();
    for part in rel
        .split(['\\', '/'])
        .map(str::trim)
        .filter(|p| !p.is_empty() && *p != ".")
    {
        if part == ".." {
            cur.pop();
            continue;
        }
        let want = part.to_lowercase();
        let Some(next) = std::fs::read_dir(&cur).ok().and_then(|rd| {
            rd.flatten()
                .find(|e| e.file_name().to_string_lossy().to_lowercase() == want)
        }) else {
            return false;
        };
        cur = next.path();
    }
    true
}

/// `(extension, keyword, also look in "model/")`.
const REFERENCES: [(&str, &str, bool); 4] = [
    ("ovh", "couple_back", false),
    ("bus", "couple_back", false),
    ("ovh", "sound", false),
    ("sco", "mesh", true),
];

fn reference_check(root: &Path, texts: &[(String, String, PathBuf)]) {
    let _ = root;
    println!("referenced files that exist, keyword at column 0 versus indented:");
    for (ext, kw, model) in REFERENCES {
        let mut n = [0usize; 2];
        let mut hit = [0usize; 2];
        for (e, text, path) in texts {
            if e != ext {
                continue;
            }
            let dir = path.parent().unwrap_or(Path::new("."));
            let lines: Vec<&str> = text.lines().collect();
            for (i, l) in lines.iter().enumerate() {
                if raw_keyword(l).as_deref() != Some(kw) {
                    continue;
                }
                let Some(value) = lines.get(i + 1).map(|v| v.trim()).filter(|v| !v.is_empty())
                else {
                    continue;
                };
                let k = usize::from(!l.starts_with('['));
                n[k] += 1;
                hit[k] += usize::from(
                    exists_ci(dir, value) || (model && exists_ci(&dir.join("model"), value)),
                );
            }
        }
        println!(
            "  .{ext} [{kw}]: column 0 {}/{} ({:.1} %), indented {}/{} ({:.1} %)",
            hit[0],
            n[0],
            100.0 * hit[0] as f64 / n[0].max(1) as f64,
            hit[1],
            n[1],
            100.0 * hit[1] as f64 / n[1].max(1) as f64
        );
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let root = PathBuf::from(args.next().expect("omsi root"));
    let exts: Vec<String> = args
        .next()
        .unwrap_or_else(|| "sco,bus,ovh,cfg,sli,con,hum,ait,txt,dsc".into())
        .split(',')
        .map(str::to_owned)
        .collect();
    let mut files = Vec::new();
    walk(&root, &exts, &mut files);
    let mut stats: BTreeMap<String, Stats> = BTreeMap::new();
    let mut column0: HashMap<String, HashSet<String>> = HashMap::new();
    let mut texts: Vec<(String, String, PathBuf)> = Vec::new();
    for p in &files {
        let ext = p
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        let Ok(bytes) = std::fs::read(p) else {
            continue;
        };
        let text = decode_bytes(&bytes);
        for l in text.lines() {
            if l.starts_with('[') {
                if let Some(k) = raw_keyword(l) {
                    column0.entry(ext.clone()).or_default().insert(k);
                }
            }
        }
        texts.push((ext, text, p.clone()));
    }
    for (ext, text, _) in &texts {
        let s = stats.entry(ext.clone()).or_default();
        s.files += 1;
        let mut raw: HashMap<String, usize> = HashMap::new();
        let mut col0_here: HashSet<String> = HashSet::new();
        let mut indented_here: HashSet<String> = HashSet::new();
        for l in text.lines() {
            if let Some(k) = raw_keyword(l) {
                *raw.entry(k.clone()).or_default() += 1;
                if l.starts_with('[') {
                    col0_here.insert(k);
                } else {
                    indented_here.insert(k.clone());
                    if !column0.get(ext).is_some_and(|c| c.contains(&k)) {
                        *s.foreign.entry(k).or_default() += 1;
                    }
                }
            }
        }
        let mut parsed: HashMap<String, usize> = HashMap::new();
        for b in scan_blocks(text) {
            *parsed.entry(b.keyword).or_default() += 1;
        }
        s.raw_keywords += raw.values().sum::<usize>();
        s.parsed_keywords += parsed.values().sum::<usize>();
        if !indented_here.is_empty() {
            s.files_indented += 1;
        }
        for (k, n) in &raw {
            let lost = n.saturating_sub(parsed.get(k).copied().unwrap_or(0));
            if lost > 0 {
                *s.lost.entry(k.clone()).or_default() += lost;
            }
        }
        for k in indented_here.difference(&col0_here) {
            *s.only_indented.entry(k.clone()).or_default() += 1;
        }
    }
    reference_check(&root, &texts);
    for (ext, s) in &stats {
        println!(
            "== .{ext}: {} files, {} with an indented keyword; keyword lines raw {} / parsed {} (lost {})",
            s.files,
            s.files_indented,
            s.raw_keywords,
            s.parsed_keywords,
            s.raw_keywords.saturating_sub(s.parsed_keywords)
        );
        let top = |m: &BTreeMap<String, usize>, n: usize| {
            let mut v: Vec<_> = m.iter().collect();
            v.sort_by_key(|(_, c)| std::cmp::Reverse(**c));
            v.into_iter()
                .take(n)
                .map(|(k, c)| format!("{k} {c}"))
                .collect::<Vec<_>>()
                .join(", ")
        };
        println!("   lost keywords:        {}", top(&s.lost, 12));
        println!(
            "   only ever indented in the file: {}",
            top(&s.only_indented, 12)
        );
        println!(
            "   indented names never at column 0 in .{ext} (likely comments): {} distinct, {} lines: {}",
            s.foreign.len(),
            s.foreign.values().sum::<usize>(),
            top(&s.foreign, 8)
        );
    }
}
