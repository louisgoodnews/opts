//! `.sco` paths referenced by the map tiles, as objects and as spline attachments.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use opts_core::formats::map::MapTile;

use super::super::normalise_rel;

/// Lower-case `.sco` paths referenced by map tiles: `(as [object], as spline attachment)`.
#[must_use]
pub fn map_references(root: &Path) -> (HashSet<String>, HashSet<String>) {
    let tiles = tile_files(root);
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get().min(8));
    let chunk = tiles.len().div_ceil(threads).max(1);
    let parts: Vec<(HashSet<String>, HashSet<String>)> = std::thread::scope(|s| {
        let handles: Vec<_> = tiles
            .chunks(chunk)
            .map(|c| s.spawn(move || scan_tiles(c)))
            .collect();
        handles.into_iter().filter_map(|h| h.join().ok()).collect()
    });
    parts.into_iter().fold(
        (HashSet::new(), HashSet::new()),
        |(mut a, mut b), (x, y)| {
            a.extend(x);
            b.extend(y);
            (a, b)
        },
    )
}

fn tile_files(root: &Path) -> Vec<PathBuf> {
    let Some(maps) = std::fs::read_dir(root).ok().and_then(|rd| {
        rd.flatten().map(|e| e.path()).find(|p| {
            p.file_name()
                .is_some_and(|n| n.to_string_lossy().eq_ignore_ascii_case("maps"))
        })
    }) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for map in std::fs::read_dir(maps).into_iter().flatten().flatten() {
        for f in std::fs::read_dir(map.path())
            .into_iter()
            .flatten()
            .flatten()
        {
            let name = f.file_name().to_string_lossy().to_lowercase();
            if name.starts_with("tile_") && name.ends_with(".map") {
                out.push(f.path());
            }
        }
    }
    out
}

fn scan_tiles(files: &[PathBuf]) -> (HashSet<String>, HashSet<String>) {
    let (mut objects, mut attached) = (HashSet::new(), HashSet::new());
    let norm = |s: &str| normalise_rel(s.trim()).to_lowercase();
    for f in files {
        let Some(tile) = std::fs::read(f).ok().and_then(|b| MapTile::parse(&b).ok()) else {
            tracing::warn!(tile = %f.display(), "map tile unreadable; its objects are not counted as referenced");
            continue;
        };
        objects.extend(tile.objects.iter().map(|o| norm(&o.sco_file)));
        attached.extend(tile.spline_attachments.iter().map(|a| norm(&a.sco_file)));
    }
    (objects, attached)
}
