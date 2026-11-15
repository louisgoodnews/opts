//! Folder lookups of the repaints: case-insensitive directory walks and texture folders.

use std::path::{Path, PathBuf};

/// Every directory `rel` (backslash or slash separated) names below `base`, comparing names
/// without regard to case: an installation can hold `texture` and `Texture` side by side (they are
/// one folder on Windows, where OMSI runs), and the repaints are in both.
#[must_use]
pub fn dirs_ignoring_case(base: &Path, rel: &str) -> Vec<PathBuf> {
    let mut level = vec![base.to_path_buf()];
    for part in rel
        .split(['\\', '/'])
        .filter(|p| !p.is_empty() && *p != ".")
    {
        let mut next = Vec::new();
        for dir in &level {
            if part == ".." {
                next.extend(dir.parent().map(Path::to_path_buf));
                continue;
            }
            let Ok(entries) = std::fs::read_dir(dir) else {
                continue;
            };
            let mut hits: Vec<PathBuf> = entries
                .flatten()
                .filter(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case(part))
                .map(|e| e.path())
                .filter(|p| p.is_dir())
                .collect();
            hits.sort();
            next.extend(hits);
        }
        level = next;
    }
    level
}

/// The texture folders of a vehicle: every `texture` folder (whatever the case) of its folder and
/// of its model's, the vehicle's `texture` first; the folder of that name at least.
#[must_use]
pub fn texture_dirs(dir: &Path, model_dir: &Path) -> Vec<PathBuf> {
    let mut out = dirs_ignoring_case(dir, "texture");
    for d in dirs_ignoring_case(model_dir, "texture") {
        if !out.contains(&d) {
            out.push(d);
        }
    }
    if out.is_empty() {
        out.push(dir.join("texture"));
    }
    out
}
