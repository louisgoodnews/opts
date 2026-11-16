//! Corpus validation: the live installation's maps, scenery and splines parse cleanly. Skipped
//! unless `OMSI_PATH` points at an installation.

use super::*;

#[test]
fn test_omsi2_corpus_live_validation() {
    let Some(omsi_root) = std::env::var_os("OMSI_PATH")
        .map(std::path::PathBuf::from)
        .filter(|p| p.exists())
    else {
        eprintln!("OMSI_PATH not set; skipping live corpus verification");
        return;
    };
    let omsi_root = omsi_root.as_path();

    // 1. Verify Grundorf global.cfg
    let grundorf_global = omsi_root.join("maps/Grundorf/global.cfg");
    if grundorf_global.exists() {
        let bytes = std::fs::read(&grundorf_global).expect("read Grundorf global.cfg");
        let manifest = MapManifest::parse(&bytes).expect("parse Grundorf global.cfg");
        assert_eq!(manifest.name, "Grundorf");
        assert!(!manifest.tiles.is_empty());
    }

    // 2. Verify Grundorf tile_0_0.map & terrain
    let grundorf_tile = omsi_root.join("maps/Grundorf/tile_0_0.map");
    if grundorf_tile.exists() {
        let bytes = std::fs::read(&grundorf_tile).expect("read Grundorf tile_0_0.map");
        let tile = MapTile::parse(&bytes).expect("parse Grundorf tile_0_0.map");
        assert!(tile.version.is_some());
        assert!(!tile.objects.is_empty() || !tile.splines.is_empty());
    }

    let grundorf_terrain = omsi_root.join("maps/Grundorf/tile_0_0.map.terrain");
    if grundorf_terrain.exists() {
        let bytes = std::fs::read(&grundorf_terrain).expect("read Grundorf terrain");
        let terrain = MapTerrain::parse(&bytes).expect("parse Grundorf terrain");
        assert_eq!(terrain.grid_cells, 60);
        assert_eq!(terrain.heights.len(), 61 * 61);
    }

    // 3. Verify Berlin-Spandau global.cfg if present
    let spandau_global = omsi_root.join("maps/Berlin-Spandau/global.cfg");
    if spandau_global.exists() {
        let bytes = std::fs::read(&spandau_global).expect("read Spandau global.cfg");
        let manifest = MapManifest::parse(&bytes).expect("parse Spandau global.cfg");
        assert_eq!(manifest.name, "Berlin-Spandau");
        assert!(manifest.tiles.len() > 10);
    }

    // 4. Sample real .sco files
    let sco_dir = omsi_root.join("Sceneryobjects");
    if sco_dir.exists() {
        let mut tested = 0;
        for entry in walkdir(sco_dir, 50, ".sco") {
            let bytes = std::fs::read(&entry).expect("read sco");
            let _ = SceneryObject::parse(&bytes).expect("sco should parse cleanly");
            tested += 1;
        }
        assert!(tested > 0, "should test at least one .sco file");
    }

    // 5. Sample real .sli files
    let sli_dir = omsi_root.join("Splines");
    if sli_dir.exists() {
        let mut tested = 0;
        for entry in walkdir(sli_dir, 50, ".sli") {
            let bytes = std::fs::read(&entry).expect("read sli");
            let _ = SplineProfile::parse(&bytes).expect("sli should parse cleanly");
            tested += 1;
        }
        assert!(tested > 0, "should test at least one .sli file");
    }
}

fn walkdir(dir: std::path::PathBuf, limit: usize, ext: &str) -> Vec<std::path::PathBuf> {
    let mut results = Vec::new();
    let mut stack = vec![dir];
    while let Some(current) = stack.pop() {
        if let Ok(entries) = std::fs::read_dir(current) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else if path.is_file() {
                    if let Some(name) = path.to_str() {
                        if name.ends_with(ext) {
                            results.push(path);
                            if results.len() >= limit {
                                return results;
                            }
                        }
                    }
                }
            }
        }
    }
    results
}
