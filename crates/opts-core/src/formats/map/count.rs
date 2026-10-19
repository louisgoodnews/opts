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
    let k = usize::try_from(total.div_ceil(budget)).unwrap_or(usize::MAX).max(2);
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
mod tests {
    use super::*;

    fn utf16(text: &str) -> Vec<u8> {
        let mut v = vec![0xFF, 0xFE];
        for u in text.encode_utf16() {
            v.extend(u.to_le_bytes());
        }
        v
    }

    const TILE: &str = "File created with OMSI-Editor\r\n[version]\r\n3\r\n[object]\r\n0\r\nSceneryobjects\\a.sco\r\n[object]\r\n0\r\n[spline]\r\n0\r\n[spline_h]\r\n0\r\n[objects]\r\n[spline_terrain_align]\r\n1\r\n [object]\r\nx [object]\r\n";

    #[test]
    fn counts_records_that_stand_alone_on_a_line_in_every_encoding() {
        let want = TileCounts { objects: 2, splines: 2 };
        assert_eq!(count_tile(&utf16(TILE)), want);
        assert_eq!(count_tile(TILE.as_bytes()), want);
        let mut bom8 = vec![0xEF, 0xBB, 0xBF];
        bom8.extend(TILE.as_bytes());
        assert_eq!(count_tile(&bom8), want);
        // LF-only line ends and CP1252 text bytes are fine too.
        let lf = TILE.replace("\r\n", "\n").replace("a.sco", "ä.sco");
        let mut cp = lf.chars().map(|c| if c == 'ä' { 0xE4 } else { c as u8 }).collect::<Vec<u8>>();
        assert_eq!(count_tile(&cp), want);
        cp.clear();
        assert_eq!(count_tile(&cp), TileCounts::default());
    }

    #[test]
    fn a_map_that_fits_the_budget_is_counted_in_full() {
        let files: Vec<(PathBuf, u64)> = (0..10).map(|i| (PathBuf::from(format!("t{i}")), 100)).collect();
        assert_eq!(sample_plan(&files, 5_000), ((0..10).collect(), false));
    }

    #[test]
    fn a_big_map_is_sampled_across_its_size_mix_and_marked() {
        // 100 tiles of 1..=100 MB-ish sizes in name order unrelated to size.
        let files: Vec<(PathBuf, u64)> = (0..100)
            .map(|i| (PathBuf::from(format!("t{i}")), 100_000 * ((i * 37) % 100 + 1) as u64))
            .collect();
        let total: u64 = files.iter().map(|f| f.1).sum();
        let (picked, sampled) = sample_plan(&files, total / 5);
        assert!(sampled && picked.windows(2).all(|w| w[0] < w[1]));
        assert!(picked.len() >= 15 && picked.len() <= 25, "{}", picked.len());
        let bytes: u64 = picked.iter().map(|i| files[*i].1).sum();
        // The sample holds about a fifth of the bytes, from small and large tiles alike.
        assert!((bytes as f64 / total as f64 - 0.2).abs() < 0.05, "{}", bytes as f64 / total as f64);
        let sizes: Vec<u64> = picked.iter().map(|i| files[*i].1).collect();
        assert!(sizes.iter().min().unwrap() < &2_000_000 && sizes.iter().max().unwrap() > &8_000_000);
    }

    #[test]
    fn a_sampled_count_estimates_the_full_count() {
        let dir = std::env::temp_dir().join(format!("opts-count-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        // 40 tiles with 5 + (i % 3) objects each and 2 splines.
        let mut full = 0;
        for i in 0..40 {
            let n = 5 + i % 3;
            full += n;
            let mut t = String::from("banner\r\n[version]\r\n3\r\n");
            for _ in 0..n {
                t.push_str("[object]\r\n0\r\n");
            }
            t.push_str("[spline]\r\n0\r\n[spline]\r\n0\r\n");
            fs::write(dir.join(format!("tile_{i}_0.map")), utf16(&t)).unwrap();
        }
        fs::write(dir.join("tile_0_0.map.terrain"), b"not a tile").unwrap();
        let exact = count_map(&dir, u64::MAX);
        assert_eq!((exact.tiles, exact.objects, exact.splines, exact.sampled), (40, full, 80, false));
        let est = count_map(&dir, 2_000);
        assert!(est.sampled && est.tiles == 40);
        let err = (est.objects as f64 - full as f64).abs() / full as f64;
        assert!(err < 0.1, "estimate {} vs {full}", est.objects);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_ai_pool_counts_distinct_vehicles_across_groups() {
        let dir = std::env::temp_dir().join(format!("opts-aipool-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let cfg = "[aigroup_2]\nNormalCars\nVehicles\\A\\a.bus\t2\nVehicles\\B\\b.bus\n[end]\n[aigroup_2]\nTrucks\nvehicles\\a\\A.bus\nVehicles\\C\\c.ovh\n[end]\n";
        fs::write(dir.join("ailists.cfg"), cfg).unwrap();
        assert_eq!(ai_pool_size(&dir), 3);
        assert_eq!(ai_pool_size(&dir.join("missing")), 0);
        let _ = fs::remove_dir_all(&dir);
    }
}
