//! Every `.map.water` file of an OMSI installation parses, and the four corner levels are finite
//! (`OPTS_OMSI_PATH`, default `/home/louisgoodnews/nvme2n1/steamapps/common/OMSI 2`; skipped without
//! an installation). Format: `docs/src/formats/map/tiles.md`.

use std::path::PathBuf;

use opts_core::formats::map::MapWater;

#[test]
fn all_water_files_of_the_installation_parse() {
    let root = std::env::var("OPTS_OMSI_PATH")
        .map_or_else(|_| PathBuf::from("/home/louisgoodnews/nvme2n1/steamapps/common/OMSI 2"), PathBuf::from);
    let maps = root.join("maps");
    let Ok(rd) = std::fs::read_dir(&maps) else {
        eprintln!("no OMSI installation: skipped");
        return;
    };
    let (mut files, mut bad) = (0usize, Vec::new());
    for map in rd.flatten() {
        let Ok(inner) = std::fs::read_dir(map.path()) else { continue };
        for f in inner.flatten() {
            if !f.file_name().to_string_lossy().to_lowercase().ends_with(".map.water") {
                continue;
            }
            files += 1;
            match MapWater::parse(&std::fs::read(f.path()).unwrap()) {
                Ok(w) if w.version == 1 && w.levels.iter().all(|l| l.is_finite() && l.abs() < 10_000.0) => {}
                Ok(w) => bad.push(format!("{}: {w:?}", f.path().display())),
                Err(e) => bad.push(format!("{}: {e}", f.path().display())),
            }
        }
    }
    eprintln!("{files} water files parsed");
    assert!(bad.is_empty(), "{} unusable water files, first: {:?}", bad.len(), bad.first());
}
