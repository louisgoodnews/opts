//! Walking a Chrono folder: finding the entries and their files.

use super::*;

/// Files of a folder by lower-case name (sorted, files only).
pub(super) fn files_of(dir: &Path) -> BTreeMap<String, PathBuf> {
    std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().is_file())
        .map(|e| (e.file_name().to_string_lossy().to_lowercase(), e.path()))
        .collect()
}

/// Collects the entries below `dir` (`rel`: its path below `Chrono/`).
pub(super) fn walk(dir: &Path, rel: &str, out: &mut Vec<ChronoEntry>) {
    let Ok(read) = std::fs::read_dir(dir) else {
        return;
    };
    let mut items: Vec<(String, PathBuf, bool)> = read
        .flatten()
        .map(|e| {
            let p = e.path();
            (
                e.file_name().to_string_lossy().into_owned(),
                p.clone(),
                p.is_dir(),
            )
        })
        .collect();
    items.sort();
    let cfg = items
        .iter()
        .find(|(n, _, d)| !d && n.eq_ignore_ascii_case("chrono.cfg"))
        .map(|(_, p, _)| p.clone());
    let mut ttdata_dir = None;
    if let Some(cfg) = cfg {
        let mut entry = ChronoEntry {
            dir: dir.to_path_buf(),
            rel: rel.to_string(),
            ..ChronoEntry::default()
        };
        if let Ok(bytes) = std::fs::read(&cfg) {
            entry.read_cfg(&bytes);
        }
        for (name, path, is_dir) in &items {
            let lower = name.to_lowercase();
            if *is_dir {
                if lower == "ttdata" {
                    entry.ttdata = files_of(path);
                    ttdata_dir = Some(path.clone());
                }
                continue;
            }
            if lower == "chrono.cfg" {
                continue;
            }
            if let Some(key) = tile_key_of(&lower) {
                entry.tiles.insert(key, path.clone());
            } else if let Some(key) = lower
                .strip_suffix(".chronoterrrelevant")
                .and_then(tile_key_of)
            {
                entry.terrain_relevant.insert(key);
            } else if let Some(lang) = lower
                .strip_prefix("chrono_")
                .and_then(|l| l.strip_suffix(".dsc"))
            {
                let mut dsc = ChronoEntry::default();
                if let Ok(bytes) = std::fs::read(path) {
                    dsc.read_cfg(&bytes);
                }
                if let Some(n) = dsc.name {
                    entry.localized_names.push((lang.to_uppercase(), n));
                }
            } else {
                entry.files.insert(lower, path.clone());
            }
        }
        out.push(entry);
    }
    for (name, path, is_dir) in items {
        if is_dir && Some(&path) != ttdata_dir.as_ref() {
            let sub = if rel.is_empty() {
                name
            } else {
                format!("{rel}/{name}")
            };
            walk(&path, &sub, out);
        }
    }
}
