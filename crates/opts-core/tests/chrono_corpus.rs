//! Corpus test of Chrono (`docs/src/formats/map/chrono.md`): activation reproduces the masks
//! cached in `timeline.prt.cfg`, overlay selections resolve when the entries are applied in
//! order, and an element created by one entry is applied before the entries that edit it.
//! Skipped unless `OMSI_PATH` points at an installation.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use opts_core::formats::chrono::{ChronoDate, ChronoSet, OverlayOp, TileOverlay};
use opts_core::formats::map::MapTile;
use opts_core::formats::text::decode_bytes;

fn maps_with_chrono(root: &Path) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = std::fs::read_dir(root.join("maps"))
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    out.sort();
    out
}

/// `(rel path below Chrono/, lower-case)` of a `timeline.prt.cfg` path such as
/// `maps\Berlin-Spandau\Chrono\0100_Neuer13N\Chrono.cfg`.
fn rel_of_timeline_path(p: &str) -> Option<String> {
    let lower = p.replace('\\', "/").to_lowercase();
    let start = lower.find("/chrono/")? + "/chrono/".len();
    let rest = lower[start..].strip_suffix("/chrono.cfg")?;
    Some(rest.to_string())
}

#[test]
fn activation_reproduces_the_cached_timeline_masks() {
    let Some(root) = std::env::var_os("OMSI_PATH").map(PathBuf::from) else {
        eprintln!("OMSI_PATH not set; skipping corpus test");
        return;
    };
    let (mut rows, mut equal, mut maps, mut stale) = (0, 0, 0, 0);
    for map in maps_with_chrono(&root) {
        let Ok(bytes) = std::fs::read(map.join("timeline.prt.cfg")) else {
            continue;
        };
        let text = decode_bytes(&bytes);
        let lines: Vec<&str> = text.lines().map(str::trim).collect();
        let Some(hash) = lines.iter().position(|l| *l == "#") else {
            continue;
        };
        let set = ChronoSet::scan(&map);
        let by_rel: HashMap<String, usize> = set
            .entries()
            .iter()
            .enumerate()
            .map(|(i, e)| (e.rel.to_lowercase(), i))
            .collect();
        let columns: Option<Vec<usize>> = lines[..hash]
            .iter()
            .step_by(2)
            .map(|p| rel_of_timeline_path(p).and_then(|r| by_rel.get(&r).copied()))
            .collect();
        let Some(columns) = columns else {
            stale += 1;
            continue;
        };
        maps += 1;
        for pair in lines[hash + 1..].chunks(2) {
            let [day, mask] = pair else { continue };
            let Ok(day) = day.parse::<i64>() else {
                continue;
            };
            // Day 1 = 2000-01-01; the mask holds up to the day before the boundary.
            let date = ChronoDate::from_unix_days(10_957 + day - 1);
            let active = set.active(date);
            let ours: String = columns
                .iter()
                .map(|&i| if active.contains(i) { '1' } else { '0' })
                .collect();
            rows += 1;
            if ours == *mask {
                equal += 1;
            } else {
                eprintln!("{}: {date}: cached {mask}, ours {ours}", map.display());
            }
        }
    }
    eprintln!(
        "timeline masks: {equal} of {rows} rows equal on {maps} maps ({stale} with stale paths)"
    );
    if rows > 0 {
        assert!(maps >= 30, "{maps} maps");
        assert!(equal * 100 >= rows * 99, "{equal} of {rows}");
    }
}

#[test]
fn overlay_selections_resolve_in_application_order() {
    let Some(root) = std::env::var_os("OMSI_PATH").map(PathBuf::from) else {
        eprintln!("OMSI_PATH not set; skipping corpus test");
        return;
    };
    let (mut entries, mut selections, mut resolved, mut tiles) = (0usize, 0usize, 0usize, 0usize);
    let (mut created_earlier, mut created_later) = (0usize, 0usize);
    for map in maps_with_chrono(&root) {
        let set = ChronoSet::scan(&map);
        if set.entries().is_empty() {
            continue;
        }
        let base_files: HashMap<String, PathBuf> = std::fs::read_dir(&map)
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| (e.file_name().to_string_lossy().to_lowercase(), e.path()))
            .collect();
        let mut base_cache: HashMap<(i32, i32), MapTile> = HashMap::new();
        let mut overlay_cache: HashMap<PathBuf, TileOverlay> = HashMap::new();
        let mut overlay = |p: &Path| -> TileOverlay {
            overlay_cache
                .entry(p.to_path_buf())
                .or_insert_with(|| TileOverlay::parse(&std::fs::read(p).unwrap_or_default()))
                .clone()
        };
        // Who creates which ID on which tile: (tile, id) -> entry indices.
        let mut creators: BTreeMap<((i32, i32), u64), Vec<usize>> = BTreeMap::new();
        for (i, e) in set.entries().iter().enumerate() {
            for (key, path) in &e.tiles {
                for op in &overlay(path).ops {
                    if let OverlayOp::Add(t) = op {
                        let ids = t
                            .objects
                            .iter()
                            .map(|o| o.id_code)
                            .chain(t.splines.iter().map(|s| s.id_code))
                            .chain(t.attached_objects.iter().map(|a| a.id_code))
                            .chain(t.spline_attachments.iter().map(|a| a.id_code));
                        for id in ids {
                            creators.entry((*key, id)).or_default().push(i);
                        }
                    }
                }
            }
        }
        for (i, e) in set.entries().iter().enumerate() {
            let date = e.start.unwrap_or_else(|| ChronoDate::from_ymd(0, 1, 1));
            let active = set.active(date);
            if !active.contains(i) {
                continue;
            }
            entries += 1;
            let plan = set.plan(&active);
            for (key, path) in &e.tiles {
                tiles += 1;
                let base = base_cache
                    .entry(*key)
                    .or_insert_with(|| {
                        let name = format!("tile_{}_{}.map", key.0, key.1);
                        base_files
                            .get(&name)
                            .and_then(|p| std::fs::read(p).ok())
                            .and_then(|b| MapTile::parse(&b).ok())
                            .unwrap_or_default()
                    })
                    .clone();
                let mut tile = base.clone();
                for p in plan.overlays(*key) {
                    let ov = overlay(p);
                    let report = tile.apply_overlay(&ov);
                    if p == path {
                        selections += report.selections;
                        resolved += report.resolved;
                        // Selections of elements created by another entry: is the creator
                        // applied before this one?
                        for op in &ov.ops {
                            let id = match op {
                                OverlayOp::SelectObject(id) | OverlayOp::SelectSpline(id) => *id,
                                _ => continue,
                            };
                            let in_base = base.objects.iter().any(|o| o.id_code == id)
                                || base.splines.iter().any(|s| s.id_code == id)
                                || base.attached_objects.iter().any(|a| a.id_code == id)
                                || base.spline_attachments.iter().any(|a| a.id_code == id);
                            if in_base {
                                continue;
                            }
                            if let Some(c) = creators.get(&(*key, id)) {
                                if c.iter().any(|&c| c < i) {
                                    created_earlier += 1;
                                } else if !c.contains(&i) {
                                    created_later += 1;
                                }
                            }
                        }
                        break;
                    }
                }
            }
        }
    }
    eprintln!(
        "{entries} entries, {tiles} overlay tiles: {resolved} of {selections} selections resolve; created by an earlier entry {created_earlier}, by a later one only {created_later}"
    );
    if selections > 0 {
        assert!(
            resolved * 1000 >= selections * 985,
            "{resolved} of {selections}"
        );
        assert!(
            created_earlier > created_later,
            "creators are applied first"
        );
    }
}
