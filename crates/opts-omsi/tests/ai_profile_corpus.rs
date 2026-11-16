//! Corpus test: an [`AiProfile`] can be built for every vehicle of every AI group of every
//! installed map, and the fields that fall back to defaults stay within the numbers recorded in
//! `docs/src/architecture/ai-vehicle-config.md`. Skipped unless `OMSI_PATH` points at an
//! installation.

#[path = "ai_profile_corpus/stats.rs"]
mod stats;

use std::collections::BTreeSet;
use std::path::PathBuf;

use opts_omsi::formats::map::AiLists;

use stats::ProfileStats;

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
