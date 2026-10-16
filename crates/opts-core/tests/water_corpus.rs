//! Corpus test: every installed `.map.water` parses, and the corner levels are shared between
//! neighbouring tiles (`docs/src/formats/map/tiles.md`, `.map.water`).
//! Skipped unless `OMSI_PATH` points at an installation.

use std::collections::HashMap;
use std::path::PathBuf;

use opts_core::formats::map::MapWater;

fn tile_key(name: &str) -> Option<(i32, i32)> {
    let rest = name.to_ascii_lowercase();
    let rest = rest
        .strip_prefix("tile_")?
        .strip_suffix(".map.water")?
        .to_owned();
    let (x, y) = rest.rsplit_once('_')?;
    Some((x.parse().ok()?, y.parse().ok()?))
}

#[test]
fn every_water_file_parses_and_neighbours_share_their_corners() {
    let Some(root) = std::env::var_os("OMSI_PATH").map(PathBuf::from) else {
        eprintln!("OMSI_PATH not set; skipping corpus test");
        return;
    };
    let Ok(maps) = std::fs::read_dir(root.join("maps")) else {
        eprintln!("no maps folder; skipping corpus test");
        return;
    };
    let (mut files, mut east_pairs, mut east_agree, mut north_pairs, mut north_agree) =
        (0, 0, 0, 0, 0);
    for map in maps.flatten().filter(|m| m.path().is_dir()) {
        let mut tiles: HashMap<(i32, i32), MapWater> = HashMap::new();
        for f in std::fs::read_dir(map.path())
            .into_iter()
            .flatten()
            .flatten()
        {
            let Some(key) = tile_key(&f.file_name().to_string_lossy()) else {
                continue;
            };
            let bytes = std::fs::read(f.path()).expect("readable");
            let w =
                MapWater::parse(&bytes).unwrap_or_else(|e| panic!("{}: {e}", f.path().display()));
            assert_eq!(w.version, 1, "{}", f.path().display());
            assert_eq!(bytes.len(), MapWater::FILE_SIZE, "{}", f.path().display());
            assert!(w.levels.iter().all(|l| l.is_finite()));
            files += 1;
            tiles.insert(key, w);
        }
        for (&(x, y), a) in &tiles {
            if let Some(b) = tiles.get(&(x + 1, y)) {
                east_pairs += 1;
                // The east edge (corners 1, 3) of `a` is the west edge (0, 2) of `b`.
                east_agree += usize::from(
                    (a.levels[1] - b.levels[0]).abs() < 1e-3
                        && (a.levels[3] - b.levels[2]).abs() < 1e-3,
                );
            }
            if let Some(b) = tiles.get(&(x, y + 1)) {
                north_pairs += 1;
                // The north edge (corners 2, 3) of `a` is the south edge (0, 1) of `b`.
                north_agree += usize::from(
                    (a.levels[2] - b.levels[0]).abs() < 1e-3
                        && (a.levels[3] - b.levels[1]).abs() < 1e-3,
                );
            }
        }
    }
    eprintln!(
        "{files} water files; east pairs {east_agree}/{east_pairs}, north pairs {north_agree}/{north_pairs} share their corners"
    );
    // Most neighbours agree; the rest are the ends of a water body or unrelated levels.
    assert!(
        east_agree * 10 >= east_pairs * 8,
        "{east_agree}/{east_pairs}"
    );
    assert!(
        north_agree * 10 >= north_pairs * 8,
        "{north_agree}/{north_pairs}"
    );
}
