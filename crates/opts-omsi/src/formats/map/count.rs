//! The static load profile of a map: how many `[object]` and `[spline]` records its tiles hold,
//! and how many distinct vehicles its AI lists name, counted straight from the files without
//! building a tile or reading any 3D data (`docs/src/architecture/map-profiles.md` §4.2).
//!
//! Tiles are line-based keyword text (UTF-16LE with a BOM in 13,151 of 13,938 files, otherwise
//! an 8-bit encoding): a record starts with its keyword alone on a line, so the count is a line
//! scan. A big map is counted from a sample of its tiles and the result is marked estimated.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use super::lists::AiLists;

/// Bytes of tile files counted in full; above this the map is sampled.
pub const SAMPLE_BUDGET_BYTES: u64 = 24 * 1024 * 1024;

/// Records of one tile.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TileCounts {
    /// `[object]` records.
    pub objects: usize,
    /// `[spline]` and `[spline_h]` records.
    pub splines: usize,
}

/// The counts of a map.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MapCounts {
    /// Tile files of the map.
    pub tiles: usize,
    /// `[object]` records (scaled up when sampled).
    pub objects: usize,
    /// `[spline]` records (scaled up when sampled).
    pub splines: usize,
    /// Counted from a sample of the tiles.
    pub sampled: bool,
}

/// Counts the records of one tile's bytes.
#[must_use]
pub fn count_tile(bytes: &[u8]) -> TileCounts {
    let text = to_latin1(bytes);
    let mut c = TileCounts::default();
    for line in text.split(|b| *b == b'\n') {
        let line = line.strip_suffix(b"\r").unwrap_or(line);
        // Cheap reject: every keyword line starts with '['.
        if line.first() != Some(&b'[') {
            continue;
        }
        match line {
            b"[object]" => c.objects += 1,
            b"[spline]" | b"[spline_h]" => c.splines += 1,
            _ => {}
        }
    }
    c
}

/// The bytes as Latin-1 text: UTF-16LE (BOM `FF FE`) keeps the low byte of each unit (`?` above
/// 255), UTF-16BE likewise, a UTF-8 BOM is skipped, anything else is taken as it is.
fn to_latin1(bytes: &[u8]) -> Vec<u8> {
    match bytes {
        [0xFF, 0xFE, rest @ ..] => rest
            .chunks_exact(2)
            .map(|p| if p[1] == 0 { p[0] } else { b'?' })
            .collect(),
        [0xFE, 0xFF, rest @ ..] => rest
            .chunks_exact(2)
            .map(|p| if p[0] == 0 { p[1] } else { b'?' })
            .collect(),
        [0xEF, 0xBB, 0xBF, rest @ ..] => rest.to_vec(),
        _ => bytes.to_vec(),
    }
}

/// The tile files of a map folder (`tile_<x>_<y>.map`), sorted by name.
#[must_use]
pub fn tile_files(map_dir: &Path) -> Vec<(PathBuf, u64)> {
    let mut v: Vec<(PathBuf, u64)> = fs::read_dir(map_dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_ascii_lowercase();
            (name.starts_with("tile_") && name.ends_with(".map"))
                .then(|| (e.path(), e.metadata().map_or(0, |m| m.len())))
        })
        .collect();
    v.sort();
    v
}

/// Which of `files` to read: all when they fit in `budget` bytes, else every k-th one.
#[must_use]
pub fn sample_plan(files: &[(PathBuf, u64)], budget: u64) -> (Vec<usize>, bool) {
    let total: u64 = files.iter().map(|f| f.1).sum();
    if total <= budget || files.len() <= 1 {
        return ((0..files.len()).collect(), false);
    }
    let k = usize::try_from(total.div_ceil(budget))
        .unwrap_or(usize::MAX)
        .max(2);
    // Every k-th tile in order of size: the sample has the size mix of the whole map (a city
    // centre and its outskirts), whatever the tile names say.
    let mut by_size: Vec<usize> = (0..files.len()).collect();
    by_size.sort_by_key(|&i| (files[i].1, i));
    let mut picked: Vec<usize> = by_size.into_iter().skip(k / 2).step_by(k).collect();
    picked.sort_unstable();
    (picked, true)
}

/// Counts the records of a map folder, reading at most about `budget` bytes of tiles.
#[must_use]
pub fn count_map(map_dir: &Path, budget: u64) -> MapCounts {
    let files = tile_files(map_dir);
    let (picked, sampled) = sample_plan(&files, budget);
    let (mut objects, mut splines, mut read_bytes) = (0usize, 0usize, 0u64);
    for i in picked {
        let Ok(bytes) = fs::read(&files[i].0) else {
            continue;
        };
        let c = count_tile(&bytes);
        objects += c.objects;
        splines += c.splines;
        read_bytes += bytes.len() as u64;
    }
    if sampled && read_bytes > 0 {
        // Scale by bytes, not by tiles: tiles differ a lot in density (a city centre against
        // its outskirts), records per byte are far more even.
        let total: u64 = files.iter().map(|f| f.1).sum();
        let scale = |n: usize| (n as f64 * total as f64 / read_bytes as f64).round() as usize;
        objects = scale(objects);
        splines = scale(splines);
    }
    MapCounts {
        tiles: files.len(),
        objects,
        splines,
        sampled,
    }
}

/// The number of distinct vehicle files the map's `ailists.cfg` names (the pool AI traffic draws
/// from; not the number of vehicles on the road). Zero without the file.
#[must_use]
pub fn ai_pool_size(map_dir: &Path) -> usize {
    let Ok(bytes) = fs::read(map_dir.join("ailists.cfg")) else {
        return 0;
    };
    let lists = AiLists::parse(&bytes);
    let mut vehicles: BTreeSet<String> = BTreeSet::new();
    for g in &lists.groups {
        for e in &g.entries {
            vehicles.insert(e.vehicle.to_ascii_lowercase().replace('\\', "/"));
        }
    }
    vehicles.len()
}

#[cfg(test)]
mod tests;
