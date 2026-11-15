//! Overlay selections of the Chrono corpus test resolve when the entries are applied in order.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use opts_omsi::formats::chrono::{ChronoDate, ChronoSet, TileOverlay};
use opts_omsi::formats::map::MapTile;

use super::creators::{build_chrono_creators, check_selection_creator_order};
use super::maps_with_chrono;

struct ChronoCheckContext<'a> {
    _set: &'a ChronoSet,
    creators: &'a BTreeMap<((i32, i32), u64), Vec<usize>>,
    base_files: &'a HashMap<String, PathBuf>,
    base_cache: &'a mut HashMap<(i32, i32), MapTile>,
    overlay_cache: &'a mut HashMap<PathBuf, TileOverlay>,
    selections: &'a mut usize,
    resolved: &'a mut usize,
    created_earlier: &'a mut usize,
    created_later: &'a mut usize,
}

fn check_chrono_entry_tile(
    ctx: &mut ChronoCheckContext<'_>,
    i: usize,
    key: (i32, i32),
    path: &Path,
    plan: &opts_omsi::formats::chrono::ChronoPlan,
) {
    let base = ctx
        .base_cache
        .entry(key)
        .or_insert_with(|| {
            let name = format!("tile_{}_{}.map", key.0, key.1);
            ctx.base_files
                .get(&name)
                .and_then(|p| std::fs::read(p).ok())
                .and_then(|b| MapTile::parse(&b).ok())
                .unwrap_or_default()
        })
        .clone();
    let mut tile = base.clone();
    for p in plan.overlays(key) {
        let ov = ctx
            .overlay_cache
            .entry(p.to_path_buf())
            .or_insert_with(|| TileOverlay::parse(&std::fs::read(p).unwrap_or_default()))
            .clone();
        let report = tile.apply_overlay(&ov);
        if p == path {
            *ctx.selections += report.selections;
            *ctx.resolved += report.resolved;
            check_selection_creator_order(
                &base,
                &ov,
                key,
                i,
                ctx.creators,
                ctx.created_earlier,
                ctx.created_later,
            );
            break;
        }
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
        let creators = build_chrono_creators(&set, &mut overlay_cache);
        let mut ctx = ChronoCheckContext {
            _set: &set,
            creators: &creators,
            base_files: &base_files,
            base_cache: &mut base_cache,
            overlay_cache: &mut overlay_cache,
            selections: &mut selections,
            resolved: &mut resolved,
            created_earlier: &mut created_earlier,
            created_later: &mut created_later,
        };
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
                check_chrono_entry_tile(&mut ctx, i, *key, path, &plan);
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
