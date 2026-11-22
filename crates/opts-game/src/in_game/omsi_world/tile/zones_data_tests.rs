//! The tile zone feed on the installed map data (ISS-012): Berlin-Spandau's covered S-Bahn track
//! (`rail_wood_S49_tunnel.sli`, ceiling at 6 m) must yield zones from its real tile files. Needs
//! `OMSI_PATH`; without it, or without that map, the test skips. No map file is committed.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use opts_core::formats::map::MapTile;
use opts_core::formats::scenery::SplineProfile;

use super::tile_zones;

/// The parsed `.sli` at the install-relative path `rel` (backslash or slash separated).
fn profile_at(root: &Path, rel: &str) -> Option<Arc<SplineProfile>> {
    let bytes = std::fs::read(root.join(rel.replace('\\', "/"))).ok()?;
    SplineProfile::parse(&bytes).ok().map(Arc::new)
}

#[test]
fn berlin_spandau_covered_track_gives_zones_from_the_installed_tiles() {
    let Some(root) = std::env::var_os("OMSI_PATH").map(PathBuf::from) else {
        eprintln!("OMSI_PATH not set; skipping");
        return;
    };
    let Ok(entries) = std::fs::read_dir(root.join("maps/Berlin-Spandau")) else {
        eprintln!("maps/Berlin-Spandau not installed; skipping");
        return;
    };
    let mut tiles = 0;
    let mut zones = 0;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if !(name.starts_with("tile_") && name.ends_with(".map")) {
            continue;
        }
        let Ok(bytes) = std::fs::read(entry.path()) else {
            continue;
        };
        let Ok(tile) = MapTile::parse(&bytes) else {
            continue;
        };
        tiles += 1;
        // The origin only places the boxes; the count does not depend on it.
        let found = tile_zones((0, 0), &tile.splines, (0.0, 0.0), |rel| {
            profile_at(&root, rel)
        });
        zones += found.len();
    }
    assert!(tiles > 0, "no Berlin-Spandau tile parsed");
    eprintln!("Berlin-Spandau: {tiles} tiles, {zones} covered-spline zones");
    assert!(zones >= 1, "the covered S-Bahn track gives no zone");
}
