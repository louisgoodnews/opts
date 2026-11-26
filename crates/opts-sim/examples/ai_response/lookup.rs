//! Case-insensitive lookup of the OMSI files a vehicle names, and its constant files' values.

use std::path::{Path, PathBuf};

use opts_sim::script::FileRead;

use super::BusManifest;

/// Reads a script file that the lookup found: the loader only needs the outcome.
pub fn read_script(path: Option<PathBuf>) -> FileRead {
    let Some(path) = path else {
        return FileRead::NotFound;
    };
    match std::fs::read(&path) {
        Ok(bytes) => FileRead::Loaded {
            name: path.to_string_lossy().into_owned(),
            bytes,
        },
        Err(e) => FileRead::Unreadable(e.to_string()),
    }
}

pub fn find_ci(base: &Path, rel: &str) -> Option<PathBuf> {
    let mut cur = base.to_path_buf();
    for comp in rel
        .split(['\\', '/'])
        .filter(|c| !c.is_empty() && *c != ".")
    {
        if comp == ".." {
            cur = cur.parent()?.to_path_buf();
            continue;
        }
        let direct = cur.join(comp);
        if direct.exists() {
            cur = direct;
            continue;
        }
        cur = std::fs::read_dir(&cur)
            .ok()?
            .flatten()
            .find(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case(comp))?
            .path();
    }
    Some(cur)
}

/// A scalar constant of a vehicle's constant files.
pub fn engine_const(bus: &BusManifest, dir: &Path, root: &Path, name: &str) -> Option<f32> {
    bus.constfiles
        .iter()
        .rev()
        .filter_map(|c| find_ci(dir, c).or_else(|| find_ci(root, c)))
        .filter_map(|p| std::fs::read(p).ok())
        .map(|b| opts_core::formats::constfile::ConstFile::parse(&b))
        .find_map(|f| {
            f.scalars
                .iter()
                .rev()
                .find(|(k, _)| k.eq_ignore_ascii_case(name))
                .map(|(_, v)| *v)
        })
}
