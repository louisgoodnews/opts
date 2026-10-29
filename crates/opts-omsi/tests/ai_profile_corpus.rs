//! Corpus test: an [`AiProfile`] can be built for every vehicle of every AI group of every
//! installed map, and the fields that fall back to defaults stay within the numbers recorded in
//! `docs/src/architecture/ai-vehicle-config.md`. Skipped unless `OMSI_PATH` points at an
//! installation.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use opts_omsi::formats::constfile::ConstFile;
use opts_omsi::formats::map::AiLists;
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
struct ProfileStats {
    total: usize,
    missing: usize,
    broken: usize,
    fallbacks: BTreeMap<&'static str, usize>,
    rail: usize,
    air: usize,
    notes: usize,
    chains: usize,
    kinds: BTreeMap<String, usize>,
}

impl ProfileStats {
    fn process_vehicle(&mut self, root: &Path, vehicle: &str) {
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

#[test]
fn every_ai_group_vehicle_yields_a_profile() {
    let Some(root) = std::env::var_os("OMSI_PATH").map(PathBuf::from) else {
        eprintln!("OMSI_PATH not set; skipping corpus test");
        return;
    };
    let mut seen = BTreeSet::new();
    let mut stats = ProfileStats::default();
    for map in std::fs::read_dir(root.join("maps"))
        .into_iter()
        .flatten()
        .flatten()
    {
        let Ok(bytes) = std::fs::read(map.path().join("ailists.cfg")) else {
            continue;
        };
        for group in AiLists::parse(&bytes).groups {
            for entry in group.entries {
                let lower = entry.vehicle.to_lowercase();
                if !(lower.ends_with(".bus") || lower.ends_with(".ovh")) {
                    continue;
                }
                if !seen.insert(entry.vehicle.to_lowercase()) {
                    continue;
                }
                stats.process_vehicle(&root, &entry.vehicle);
            }
        }
    }
    let total = stats.total;
    let broken = stats.broken;
    eprintln!(
        "{total} vehicles, {} files missing, {broken} unparsable; fallbacks {:?}; \
         rail {}, air {}, notes {}, with trailer {}; kinds {:?}",
        stats.missing,
        stats.fallbacks,
        stats.rail,
        stats.air,
        stats.notes,
        stats.chains,
        stats.kinds
    );
    assert!(
        total > 300,
        "expected the several hundred distinct AI vehicles of the installation"
    );
    assert!(
        broken * 100 < total,
        "{broken} of {total} vehicle files do not parse"
    );
    let share = |f: &str| stats.fallbacks.get(f).copied().unwrap_or(0) * 100 / total;
    assert!(share("mass") <= 2, "mass fallbacks: {:?}", stats.fallbacks);
    assert!(
        share("top_speed") <= 30,
        "top speed fallbacks: {:?}",
        stats.fallbacks
    );
    assert!(
        share("gas_max") <= 15,
        "drive fallbacks: {:?}",
        stats.fallbacks
    );
    assert!(
        share("brake_max") <= 10,
        "brake fallbacks: {:?}",
        stats.fallbacks
    );
    assert!(
        share("wheel_radius") <= 5,
        "wheel fallbacks: {:?}",
        stats.fallbacks
    );
    assert!(stats.chains > 0, "trailers are declared in the AI groups");
}
