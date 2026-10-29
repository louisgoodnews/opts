//! Measures the AI traffic density model around the entry points of real maps, for calibrating
//! `L0` (metres of lane per vehicle at density 1) against the former fixed target.
//!
//! Usage: `cargo run --release -p opts-traffic --example density_census -- <map dir>...`
//!
//! For each entry point (up to five per map) on the 3 × 3 tiles around it, at 12:00 on a weekday:
//! * `old`: the former target `min(30, resident road length / 250 m × Σ group factors)`, factors
//!   clamped to `0..=2`, only groups with a default density of 1 or more;
//! * `S`: `Σ_groups Σ_lanes length inside 320 m × D(lane, group) × f_group`, the new model's
//!   numerator (target = `S / L0`).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use glam::DVec2;
use opts_core::formats::map::lists::{day, GroupDensity, UnschedGroup};
use opts_core::formats::map::{
    parse_unsched_trafficdens, parse_unsched_vehgroups, AiLists, DensityProfile, MapManifest,
    MapTile, TileGrid,
};
use opts_core::formats::scenery::{
    SceneryObject, ScoPath, ScoTrafficLightGroup, SplinePath, SplineProfile,
};
use opts_traffic::{
    segments_from_tile, tile_origin, ConnectParams, LaneGraph, LaneKind, PathSource,
};

fn find_ci(base: &Path, rel: &str) -> Option<PathBuf> {
    let mut cur = base.to_path_buf();
    for part in rel.split(['\\', '/']).filter(|p| !p.is_empty()) {
        let hit = std::fs::read_dir(&cur)
            .ok()?
            .flatten()
            .find(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case(part))?;
        cur = hit.path();
    }
    Some(cur)
}

struct Files {
    root: PathBuf,
    sli: HashMap<String, Option<Vec<SplinePath>>>,
    sco: HashMap<String, Option<SceneryObject>>,
}

impl PathSource for Files {
    fn sli_paths(&mut self, f: &str) -> Option<&[SplinePath]> {
        let root = &self.root;
        self.sli
            .entry(f.to_owned())
            .or_insert_with(|| {
                let bytes = std::fs::read(find_ci(root, f)?).ok()?;
                SplineProfile::parse(&bytes).ok().map(|p| p.paths)
            })
            .as_deref()
    }

    fn sco_paths(&mut self, f: &str) -> Option<&[ScoPath]> {
        let root = &self.root;
        self.sco
            .entry(f.to_owned())
            .or_insert_with(|| {
                let bytes = std::fs::read(find_ci(root, f)?).ok()?;
                SceneryObject::parse(&bytes).ok()
            })
            .as_ref()
            .map(|o| o.paths.as_slice())
    }

    fn sco_lights(&mut self, f: &str) -> Option<&[ScoTrafficLightGroup]> {
        self.sco_paths(f)?;
        self.sco
            .get(f)?
            .as_ref()
            .map(|o| o.traffic_light_groups.as_slice())
    }
}

fn read(map: &Path, name: &str) -> Option<Vec<u8>> {
    std::fs::read(find_ci(map, name)?).ok()
}

const HOUR: f32 = 12.0;
const DISC_M: f64 = 320.0;

/// What a map says about its AI traffic.
struct Traffic {
    manifest: MapManifest,
    lists: AiLists,
    vg: Vec<UnschedGroup>,
    dens: Vec<GroupDensity>,
}

/// The new model's groups with their factor, and the old model's factor sum.
fn group_weights(t: &Traffic) -> (Vec<(usize, f64)>, f64) {
    let road = DensityProfile {
        days: 0,
        points: t.manifest.traffic_density_road.clone(),
    };
    // New: every listed group present in the AI lists (else NormalCars / first as group 0).
    let mut groups: Vec<(usize, f64)> =
        t.vg.iter()
            .enumerate()
            .filter(|(_, g)| t.lists.group(&g.name).is_some())
            .map(|(i, g)| {
                let f = t
                    .dens
                    .iter()
                    .find(|d| d.name.eq_ignore_ascii_case(&g.name))
                    .map_or(road.at(HOUR), |d| d.density(day::WEEKDAY, HOUR));
                (i, f64::from(f))
            })
            .collect();
    // Old: groups with default ≥ 1, factor clamped, 1 without a curve.
    let mut old_sum: f64 =
        t.vg.iter()
            .filter(|g| g.default_density >= 1 && t.lists.group(&g.name).is_some())
            .map(|g| {
                f64::from(
                    t.dens
                        .iter()
                        .find(|d| d.name.eq_ignore_ascii_case(&g.name))
                        .map_or(1.0, |d| d.density(day::WEEKDAY, HOUR))
                        .clamp(0.0, 2.0),
                )
            })
            .sum();
    if groups.is_empty() && !t.lists.groups.is_empty() {
        groups.push((0, f64::from(road.at(HOUR))));
    }
    if old_sum == 0.0 && !t.lists.groups.is_empty() {
        old_sum = 1.0;
    }
    (groups, old_sum)
}

/// The tiles of a map, read on first use.
struct Tiles<'a> {
    map_dir: &'a Path,
    keys: HashMap<(i32, i32), String>,
    cache: HashMap<(i32, i32), Option<MapTile>>,
}

impl Tiles<'_> {
    fn load(&mut self, k: (i32, i32)) -> Option<MapTile> {
        let (keys, dir) = (&self.keys, self.map_dir);
        self.cache
            .entry(k)
            .or_insert_with(|| {
                keys.get(&k)
                    .and_then(|n| read(dir, n))
                    .and_then(|b| MapTile::parse(&b).ok())
            })
            .clone()
    }
}

/// The road lanes of the 3 x 3 tiles around tile `key`.
fn road_graph(tiles: &mut Tiles, grid: &TileGrid, files: &mut Files, key: (i32, i32)) -> LaneGraph {
    let mut segs = Vec::new();
    for dx in -1..=1 {
        for dy in -1..=1 {
            let k = (key.0 + dx, key.1 + dy);
            if let Some(tile) = tiles.load(k) {
                segs.extend(segments_from_tile(
                    &tile,
                    tile_origin(grid, k.0, k.1),
                    files,
                ));
            }
        }
    }
    segs.retain(|s| s.kind == LaneKind::Road);
    LaneGraph::build(&segs, &ConnectParams::default())
}

/// `S`: the lane length inside the disc around `centre`, weighted by density and group factor.
fn weighted_length(
    graph: &LaneGraph,
    centre: DVec2,
    groups: &[(usize, f64)],
    defaults: &[u32],
) -> f64 {
    let mut s = 0.0;
    for lane in graph.lane_ids().filter_map(|l| graph.lane(l)) {
        let len = lane.length();
        let inside = (0..=8)
            .filter(|&i| {
                lane.geometry
                    .point_at(len * f64::from(i) / 8.0)
                    .distance(centre)
                    <= DISC_M
            })
            .count();
        let metres = len * inside as f64 / 9.0;
        for &(g, f) in groups {
            s += metres * lane.rules.density.for_group(g, defaults) * f;
        }
    }
    s
}

/// The census of one map: a row per entry point.
fn census(map_dir: &Path) -> Vec<(String, f64, f64, f64)> {
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

fn main() {
    for arg in std::env::args().skip(1) {
        let map_dir = PathBuf::from(arg);
        let name = map_dir
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        for (ep, km, old, s) in census(&map_dir) {
            println!(
                "{name:<28} {ep:<32} resident {km:>6.1} km  old {old:>5.1}  S {s:>8.0}  S/30 {:>6.0}  L0=S/old {:>6.0}",
                s / 30.0,
                if old > 0.0 { s / old } else { 0.0 }
            );
        }
    }
}
