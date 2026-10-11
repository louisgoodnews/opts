//! Corpus enumeration: every `.sco` below `Sceneryobjects/`, its tier (placed by a map tile,
//! only as a spline attachment, or not at all), selection filters and `objects.jsonl`.

use std::collections::HashSet;
use std::io::{BufRead, Write as _};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use opts_core::formats::map::MapTile;

use super::{normalise_rel, object_key, ObjectEntry};

/// Lists every `.sco` file below the installation's `Sceneryobjects` folder, sorted.
///
/// # Errors
/// The folder is missing.
pub fn list_sco_files(root: &Path) -> Result<Vec<String>> {
    let dir = std::fs::read_dir(root)?
        .flatten()
        .map(|e| e.path())
        .find(|p| {
            p.is_dir()
                && p.file_name()
                    .is_some_and(|n| n.to_string_lossy().eq_ignore_ascii_case("sceneryobjects"))
        })
        .with_context(|| format!("no Sceneryobjects folder in {}", root.display()))?;
    let mut out = Vec::new();
    let mut stack = vec![dir];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else {
            continue;
        };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p
                .extension()
                .is_some_and(|x| x.to_string_lossy().eq_ignore_ascii_case("sco"))
            {
                if let Ok(rel) = p.strip_prefix(root) {
                    out.push(normalise_rel(&rel.to_string_lossy()));
                }
            }
        }
    }
    out.sort();
    Ok(out)
}

/// Lower-case `.sco` paths referenced by map tiles: `(as [object], as spline attachment)`.
#[must_use]
pub fn map_references(root: &Path) -> (HashSet<String>, HashSet<String>) {
    let tiles = tile_files(root);
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get().min(8));
    let chunk = tiles.len().div_ceil(threads).max(1);
    let parts: Vec<(HashSet<String>, HashSet<String>)> = std::thread::scope(|s| {
        let handles: Vec<_> = tiles
            .chunks(chunk)
            .map(|c| s.spawn(move || scan_tiles(c)))
            .collect();
        handles.into_iter().filter_map(|h| h.join().ok()).collect()
    });
    parts.into_iter().fold(
        (HashSet::new(), HashSet::new()),
        |(mut a, mut b), (x, y)| {
            a.extend(x);
            b.extend(y);
            (a, b)
        },
    )
}

fn tile_files(root: &Path) -> Vec<PathBuf> {
    let Some(maps) = std::fs::read_dir(root).ok().and_then(|rd| {
        rd.flatten().map(|e| e.path()).find(|p| {
            p.file_name()
                .is_some_and(|n| n.to_string_lossy().eq_ignore_ascii_case("maps"))
        })
    }) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for map in std::fs::read_dir(maps).into_iter().flatten().flatten() {
        for f in std::fs::read_dir(map.path())
            .into_iter()
            .flatten()
            .flatten()
        {
            let name = f.file_name().to_string_lossy().to_lowercase();
            if name.starts_with("tile_") && name.ends_with(".map") {
                out.push(f.path());
            }
        }
    }
    out
}

fn scan_tiles(files: &[PathBuf]) -> (HashSet<String>, HashSet<String>) {
    let (mut objects, mut attached) = (HashSet::new(), HashSet::new());
    let norm = |s: &str| normalise_rel(s.trim()).to_lowercase();
    for f in files {
        let Some(tile) = std::fs::read(f).ok().and_then(|b| MapTile::parse(&b).ok()) else {
            continue;
        };
        objects.extend(tile.objects.iter().map(|o| norm(&o.sco_file)));
        attached.extend(tile.spline_attachments.iter().map(|a| norm(&a.sco_file)));
    }
    (objects, attached)
}

/// Builds the entries with their tiers.
///
/// # Errors
/// The `Sceneryobjects` folder is missing.
pub fn entries(root: &Path) -> Result<Vec<ObjectEntry>> {
    let files = list_sco_files(root)?;
    let (objects, attached) = map_references(root);
    Ok(files
        .into_iter()
        .map(|sco| {
            let lower = sco.to_lowercase();
            let tier = if objects.contains(&lower) {
                "referenced"
            } else if attached.contains(&lower) {
                "attachment"
            } else {
                "unreferenced"
            };
            ObjectEntry {
                key: object_key(&sco),
                sco,
                tier: tier.into(),
            }
        })
        .collect())
}

/// Selection options.
#[derive(Debug, Clone, Default)]
pub struct Selection {
    /// `referenced` (placed or attached by a map) or `all`.
    pub tier: String,
    /// Case-insensitive glob on the relative path.
    pub filter: Option<String>,
    /// Explicit list of relative paths (one per line).
    pub list: Option<Vec<String>>,
    /// Every n-th object (by key order) so that about this many remain.
    pub sample: Option<usize>,
    /// At most this many objects.
    pub limit: Option<usize>,
}

/// Applies the selection.
///
/// # Errors
/// An invalid glob.
pub fn select(mut all: Vec<ObjectEntry>, sel: &Selection) -> Result<Vec<ObjectEntry>> {
    if sel.tier == "referenced" {
        all.retain(|e| e.tier != "unreferenced");
    }
    if let Some(pattern) = &sel.filter {
        let glob = globset::GlobBuilder::new(pattern)
            .case_insensitive(true)
            .literal_separator(false)
            .build()?
            .compile_matcher();
        all.retain(|e| glob.is_match(&e.sco));
    }
    if let Some(list) = &sel.list {
        let wanted: HashSet<String> = list
            .iter()
            .map(|s| normalise_rel(s).to_lowercase())
            .collect();
        all.retain(|e| wanted.contains(&e.sco.to_lowercase()));
    }
    if let Some(n) = sel.sample.filter(|n| *n > 0 && *n < all.len()) {
        all.sort_by(|a, b| a.key.cmp(&b.key));
        let step = all.len() as f64 / n as f64;
        all = (0..n)
            .filter_map(|i| all.get((i as f64 * step) as usize).cloned())
            .collect();
    }
    if let Some(n) = sel.limit {
        all.truncate(n);
    }
    Ok(all)
}

/// Writes `objects.jsonl`.
///
/// # Errors
/// I/O failures.
pub fn write_jsonl(path: &Path, entries: &[ObjectEntry]) -> Result<()> {
    let mut f = std::io::BufWriter::new(std::fs::File::create(path)?);
    for e in entries {
        writeln!(f, "{}", serde_json::to_string(e)?)?;
    }
    f.flush()?;
    Ok(())
}

/// Reads `objects.jsonl`.
///
/// # Errors
/// I/O or JSON failures.
pub fn read_jsonl(path: &Path) -> Result<Vec<ObjectEntry>> {
    let f = std::io::BufReader::new(std::fs::File::open(path)?);
    f.lines().map(|l| Ok(serde_json::from_str(&l?)?)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(sco: &str, tier: &str) -> ObjectEntry {
        ObjectEntry {
            sco: sco.into(),
            key: object_key(sco),
            tier: tier.into(),
        }
    }

    #[test]
    fn selection_filters_tier_glob_sample_and_limit() {
        let all: Vec<_> = (0..100)
            .map(|i| {
                entry(
                    &format!("Sceneryobjects/P{}/o{i}.sco", i % 3),
                    if i % 2 == 0 {
                        "referenced"
                    } else {
                        "unreferenced"
                    },
                )
            })
            .collect();
        let sel = Selection {
            tier: "referenced".into(),
            filter: Some("sceneryobjects/p0/*".into()),
            ..Selection::default()
        };
        let got = select(all.clone(), &sel).unwrap();
        assert!(got
            .iter()
            .all(|e| e.tier == "referenced" && e.sco.contains("/P0/")));
        let sampled = select(
            all,
            &Selection {
                tier: "all".into(),
                sample: Some(10),
                limit: Some(7),
                ..Selection::default()
            },
        )
        .unwrap();
        assert_eq!(sampled.len(), 7);
    }

    #[test]
    fn enumerates_sco_files_and_tiers() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("Sceneryobjects/Pack")).unwrap();
        std::fs::create_dir_all(root.join("maps/Town")).unwrap();
        std::fs::write(root.join("Sceneryobjects/Pack/a.sco"), "[mesh]\na.o3d\n").unwrap();
        std::fs::write(root.join("Sceneryobjects/Pack/b.sco"), "[mesh]\nb.o3d\n").unwrap();
        std::fs::write(
            root.join("maps/Town/tile_0_0.map"),
            "[version]\n14\n[object]\n0\nSceneryobjects\\PACK\\A.sco\n1\n0\n0\n0\n0\n0\n0\n0\n",
        )
        .unwrap();
        let e = entries(root).unwrap();
        assert_eq!(e.len(), 2);
        assert_eq!(e[0].tier, "referenced");
        assert_eq!(e[1].tier, "unreferenced");
    }
}
