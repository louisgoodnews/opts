//! Path-index bookkeeping of the tile rule corpus: case-insensitive file resolution and the
//! count of addressable paths per spline or object file (cached).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use opts_omsi::formats::scenery::{SceneryObject, SplineProfile};
pub(super) fn find_ci(base: &Path, rel: &str) -> Option<PathBuf> {
    let mut cur = base.to_path_buf();
    for part in rel.split(['\\', '/']).filter(|p| !p.is_empty()) {
        let hit = std::fs::read_dir(&cur)
            .ok()?
            .flatten()
            .find(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case(part))?;
        cur = hit.path();
    }
    Some(cur)
}

/// Number of addressable paths of a file: a spline's own; an object's own plus those of the
/// spline profiles it embeds with `[splinehelper]` (rule indices continue into them). Cached.
pub(super) struct PathCounts {
    pub(super) root: PathBuf,
    pub(super) cache: HashMap<String, Option<usize>>,
}

impl PathCounts {
    fn profile_paths(&self, file: &str) -> Option<usize> {
        let bytes = std::fs::read(find_ci(&self.root, file)?).ok()?;
        SplineProfile::parse(&bytes).ok().map(|p| p.paths.len())
    }

    pub(super) fn count(&mut self, file: &str, spline: bool) -> Option<usize> {
        let key = file.to_ascii_lowercase();
        if let Some(hit) = self.cache.get(&key) {
            return *hit;
        }
        let n = self.compute(file, spline);
        self.cache.insert(key, n);
        n
    }

    fn compute(&self, file: &str, spline: bool) -> Option<usize> {
        if spline {
            return self.profile_paths(file);
        }
        let bytes = std::fs::read(find_ci(&self.root, file)?).ok()?;
        let o = SceneryObject::parse(&bytes).ok()?;
        let helpers: usize = o
            .spline_helpers
            .iter()
            .map(|h| self.profile_paths(&h.profile_file).unwrap_or(0))
            .sum();
        Some(o.paths.len() + helpers)
    }
}
