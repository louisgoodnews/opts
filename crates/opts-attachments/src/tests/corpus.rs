//! Corpus loading for the attachment checks: the map's tiles and their spline index.

use super::super::*;
use opts_core::formats::map::MapTile;
use std::path::{Path, PathBuf};

pub(super) fn omsi() -> Option<PathBuf> {
    std::env::var_os("OMSI_PATH").map(PathBuf::from)
}

/// Every tile of a map folder with its key.
pub(super) fn tiles(map: &Path) -> Vec<((i32, i32), MapTile)> {
    let mut out = Vec::new();
    for e in std::fs::read_dir(map).into_iter().flatten().flatten() {
        let name = e.file_name().to_string_lossy().to_ascii_lowercase();
        let Some(xy) = name
            .strip_prefix("tile_")
            .and_then(|n| n.strip_suffix(".map"))
        else {
            continue;
        };
        let Some((x, y)) = xy.split_once('_') else {
            continue;
        };
        let (Ok(x), Ok(y)) = (x.parse(), y.parse()) else {
            continue;
        };
        if let Some(t) = std::fs::read(e.path())
            .ok()
            .and_then(|b| MapTile::parse(&b).ok())
        {
            out.push(((x, y), t));
        }
    }
    out.sort_by_key(|(k, _)| *k);
    out
}

pub(super) fn index_of(tiles: &[((i32, i32), MapTile)]) -> SplineIndex {
    let mut index = SplineIndex::default();
    for (key, t) in tiles {
        index.add_tile(*key, &t.splines);
    }
    index.finish();
    index
}
