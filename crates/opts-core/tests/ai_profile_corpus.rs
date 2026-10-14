//! Corpus test: an [`AiProfile`] can be built for every vehicle of every AI group of every
//! installed map, and the fields that fall back to defaults stay within the numbers recorded in
//! `docs/src/architecture/ai-vehicle-config.md`. Skipped unless `OMSI_PATH` points at an
//! installation.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use opts_core::formats::constfile::ConstFile;
use opts_core::formats::map::AiLists;
use opts_core::formats::vehicles::{AiKind, AiProfile, BusManifest, VehicleDomain};

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

#[test]
fn every_ai_group_vehicle_yields_a_profile() {
    let Some(root) = std::env::var_os("OMSI_PATH").map(PathBuf::from) else {
        eprintln!("OMSI_PATH not set; skipping corpus test");
        return;
    };
    let mut seen = BTreeSet::new();
    let (mut missing, mut broken) = (0, 0);
    let mut fallbacks: BTreeMap<&str, usize> = BTreeMap::new();
    let (mut rail, mut air, mut notes, mut chains) = (0, 0, 0, 0);
    let mut kinds: BTreeMap<String, usize> = BTreeMap::new();
    let mut total = 0;
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
                // Train consists (`.zug`) are not road vehicles; the spawner skips them as well.
                let lower = entry.vehicle.to_lowercase();
                if !(lower.ends_with(".bus") || lower.ends_with(".ovh")) {
                    continue;
                }
                if !seen.insert(entry.vehicle.to_lowercase()) {
                    continue;
                }
                let Some(path) = find(&root, &entry.vehicle) else {
                    missing += 1;
                    continue;
                };
                let Ok(manifest) = std::fs::read(&path).map(|b| BusManifest::parse_bytes(&b))
                else {
                    broken += 1;
                    continue;
                };
                let Ok(manifest) = manifest else {
                    broken += 1;
                    continue;
                };
                let dir = path.parent().unwrap_or(&root).to_path_buf();
                let consts: Vec<ConstFile> = manifest
                    .constfiles
                    .iter()
                    .filter_map(|c| find(&dir, c))
                    .filter_map(|p| std::fs::read(p).ok())
                    .map(|b| ConstFile::parse(&b))
                    .collect();
                let p = AiProfile::from_manifest(&manifest, &consts);
                total += 1;
                for f in &p.fallbacks {
                    *fallbacks.entry(f).or_default() += 1;
                }
                rail += usize::from(p.domain == VehicleDomain::Rail);
                air += usize::from(p.domain == VehicleDomain::Air);
                notes += p.notes.len();
                chains += usize::from(p.couple_back.is_some());
                *kinds
                    .entry(p.kind.map_or("none".into(), |k: AiKind| format!("{k:?}")))
                    .or_default() += 1;
            }
        }
    }
    eprintln!(
        "{total} vehicles, {missing} files missing, {broken} unparsable; fallbacks {fallbacks:?}; \
         rail {rail}, air {air}, notes {notes}, with trailer {chains}; kinds {kinds:?}"
    );
    assert!(
        total > 300,
        "expected the several hundred distinct AI vehicles of the installation"
    );
    assert!(
        broken * 100 < total,
        "{broken} of {total} vehicle files do not parse"
    );
    let share = |f: &str| fallbacks.get(f).copied().unwrap_or(0) * 100 / total;
    // The mapping can run on defaults for a field only when few vehicles lack it (measured on
    // 426 vehicles: mass 6, brake 33, drive 60, top speed 109, wheel radius 0).
    assert!(share("mass") <= 2, "mass fallbacks: {fallbacks:?}");
    assert!(
        share("top_speed") <= 30,
        "top speed fallbacks: {fallbacks:?}"
    );
    assert!(share("gas_max") <= 15, "drive fallbacks: {fallbacks:?}");
    assert!(share("brake_max") <= 10, "brake fallbacks: {fallbacks:?}");
    assert!(share("wheel_radius") <= 5, "wheel fallbacks: {fallbacks:?}");
    assert!(chains > 0, "trailers are declared in the AI groups");
}
