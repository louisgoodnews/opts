//! Corpus test: every `[rule]` of every installed tile belongs to a spline or object, and its path
//! index addresses one of that element's `[path]` records (`docs/src/formats/map/tiles.md`).
//! Skipped unless `OMSI_PATH` points at an installation. Prints the statistics quoted in the docs.

use std::collections::HashMap;
use std::path::PathBuf;

use opts_omsi::formats::map::MapTile;

#[path = "tile_rules_corpus/paths.rs"]
mod paths;
#[path = "tile_rules_corpus/stats.rs"]
mod stats;

use paths::PathCounts;
use stats::Stats;

#[test]
fn every_rule_has_an_owner_and_a_valid_path_index() {
    let Some(root) = std::env::var_os("OMSI_PATH").map(PathBuf::from) else {
        eprintln!("OMSI_PATH not set; skipping corpus test");
        return;
    };
    let maps = std::fs::read_dir(root.join("maps")).unwrap();
    let mut counts = PathCounts {
        root,
        cache: HashMap::new(),
    };
    let mut stats = Stats::default();
    for e in maps.flatten() {
        let Ok(rd) = std::fs::read_dir(e.path()) else {
            continue;
        };
        for f in rd.flatten() {
            let name = f.file_name().to_string_lossy().to_lowercase();
            if !(name.starts_with("tile_") && name.ends_with(".map")) {
                continue;
            }
            let Some(tile) = std::fs::read(f.path())
                .ok()
                .and_then(|b| MapTile::parse(&b).ok())
            else {
                continue;
            };
            stats.tiles += 1;
            for r in &tile.rules {
                stats.rule(&tile, r, &mut counts);
            }
        }
    }
    stats.report();
    assert_eq!(
        stats.no_owner, 0,
        "every rule should follow a spline or object"
    );
}
