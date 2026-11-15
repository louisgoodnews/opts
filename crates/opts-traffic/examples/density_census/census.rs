//! The census of one map: one row per entry point.

use std::collections::HashMap;
use std::path::Path;

use glam::DVec2;
use opts_core::formats::map::{
    parse_unsched_trafficdens, parse_unsched_vehgroups, AiLists, MapManifest, TileGrid,
};
use opts_traffic::tile_origin;

use crate::files::{read, Files};
use crate::tiles::{road_graph, Tiles};
use crate::traffic::{group_weights, Traffic};
use crate::weights::weighted_length;

/// The census of one map: a row per entry point.
pub(crate) fn census(map_dir: &Path) -> Vec<(String, f64, f64, f64)> {
    let root = map_dir
        .parent()
        .and_then(Path::parent)
        .expect("OMSI root")
        .to_path_buf();
    let Some(manifest) = read(map_dir, "global.cfg").and_then(|b| MapManifest::parse(&b).ok())
    else {
        return Vec::new();
    };
    let t = Traffic {
        lists: read(map_dir, "ailists.cfg")
            .map(|b| AiLists::parse(&b))
            .unwrap_or_default(),
        vg: read(map_dir, "unsched_vehgroups.txt")
            .map(|b| parse_unsched_vehgroups(&b))
            .unwrap_or_default(),
        dens: read(map_dir, "unsched_trafficdens.txt")
            .map(|b| parse_unsched_trafficdens(&b))
            .unwrap_or_default(),
        manifest,
    };
    let defaults: Vec<u32> = if t.vg.is_empty() {
        vec![1]
    } else {
        t.vg.iter().map(|g| g.default_density).collect()
    };
    let (groups, old_sum) = group_weights(&t);
    let mut files = Files {
        root,
        sli: HashMap::new(),
        sco: HashMap::new(),
    };
    let mut tiles = Tiles {
        map_dir,
        keys: t
            .manifest
            .tiles
            .iter()
            .map(|m| ((m.x, m.y), m.file_name.clone()))
            .collect(),
        cache: HashMap::new(),
    };
    let sample: Vec<_> = t
        .manifest
        .tiles
        .iter()
        .take(80)
        .filter_map(|m| tiles.load((m.x, m.y)).map(|x| ((m.x, m.y), x.splines)))
        .collect();
    let (grid, _) = TileGrid::measure(&sample, t.manifest.world_coordinates);
    let mut rows = Vec::new();
    for ep in t.manifest.entry_points.iter().take(5) {
        let Some((key, pos)) = t.manifest.tiles.iter().find_map(|m| {
            let tile = tiles.load((m.x, m.y))?;
            let o = tile
                .objects
                .iter()
                .find(|o| o.id_code == u64::from(ep.id_code))?;
            Some(((m.x, m.y), o.position))
        }) else {
            continue;
        };
        let graph = road_graph(&mut tiles, &grid, &mut files, key);
        let centre =
            tile_origin(&grid, key.0, key.1) + DVec2::new(f64::from(pos.x), f64::from(pos.y));
        let resident: f64 = graph
            .lane_ids()
            .filter_map(|l| graph.lane(l))
            .map(|l| l.length())
            .sum();
        let old = (resident / 250.0 * old_sum).min(30.0);
        let s = weighted_length(&graph, centre, &groups, &defaults);
        rows.push((ep.name.clone(), resident / 1000.0, old, s));
    }
    rows
}
