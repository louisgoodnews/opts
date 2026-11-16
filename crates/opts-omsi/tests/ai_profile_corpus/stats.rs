//! Corpus statistics of the AI vehicle profiles: the path resolution of the installation
//! and the per-vehicle tallies that the corpus test checks.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use opts_omsi::formats::constfile::ConstFile;
use opts_omsi::formats::vehicles::{AiKind, AiProfile, BusManifest, VehicleDomain};
/// Resolves a path with `\` or `/` separators and any letter case below `base`.
fn find(base: &Path, rel: &str) -> Option<PathBuf> {
    let mut cur = base.to_path_buf();
    for part in rel
        .split(['\\', '/'])
        .filter(|p| !p.is_empty() && *p != ".")
    {
        if part == ".." {
            cur = cur.parent()?.to_path_buf();
            continue;
        }
        let exact = cur.join(part);
        if exact.exists() {
            cur = exact;
            continue;
        }
        let lower = part.to_lowercase();
        cur = std::fs::read_dir(&cur)
            .ok()?
            .flatten()
            .find(|e| e.file_name().to_string_lossy().to_lowercase() == lower)?
            .path();
    }
    Some(cur)
}

#[derive(Default)]
pub(super) struct ProfileStats {
    pub(super) total: usize,
    pub(super) missing: usize,
    pub(super) broken: usize,
    pub(super) fallbacks: BTreeMap<&'static str, usize>,
    pub(super) rail: usize,
    pub(super) air: usize,
    pub(super) notes: usize,
    pub(super) chains: usize,
    pub(super) kinds: BTreeMap<String, usize>,
}

impl ProfileStats {
    pub(super) fn process_vehicle(&mut self, root: &Path, vehicle: &str) {
        let Some(path) = find(root, vehicle) else {
            self.missing += 1;
            return;
        };
        let Ok(manifest) = std::fs::read(&path).map(|b| BusManifest::parse_bytes(&b)) else {
            self.broken += 1;
            return;
        };
        let Ok(manifest) = manifest else {
            self.broken += 1;
            return;
        };
        let dir = path.parent().unwrap_or(root).to_path_buf();
        let consts: Vec<ConstFile> = manifest
            .constfiles
            .iter()
            .filter_map(|c| find(&dir, c))
            .filter_map(|p| std::fs::read(p).ok())
            .map(|b| ConstFile::parse(&b))
            .collect();
        let p = AiProfile::from_manifest(&manifest, &consts);
        self.total += 1;
        for f in &p.fallbacks {
            *self.fallbacks.entry(f).or_default() += 1;
        }
        self.rail += usize::from(p.domain == VehicleDomain::Rail);
        self.air += usize::from(p.domain == VehicleDomain::Air);
        self.notes += p.notes.len();
        self.chains += usize::from(p.couple_back.is_some());
        *self
            .kinds
            .entry(p.kind.map_or("none".into(), |k: AiKind| format!("{k:?}")))
            .or_default() += 1;
    }
}
