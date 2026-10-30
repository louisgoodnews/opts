//! How much of a map's drawn splines does the terrain cover? Rebuilds every tile's terrain as the
//! game does (`carve_terrain` under the ground surfaces of the tile's and its neighbours'
//! splines, on the map's `TileGrid`) and walks the centreline of every drawn spline in 1 m steps:
//! a sample counts as covered when the carved terrain of the tile it lies in is more than 5 cm
//! above the profile's centre surface. Samples are split by whether they lie in the spline's own
//! tile or in a neighbour, and by whether the profile has a ground surface.
//!
//! Usage: `cargo run --release -p opts-bevy --example spline_terrain_cover -- <map dir>`
use std::collections::{BTreeMap, HashMap};

use opts_bevy::install::OmsiInstall;
use opts_bevy::scenery::{extrude_spline_segment, spline_pose};
use opts_bevy::terrain::{carve_terrain, spline_ground_triangles, TileTerrain};
use opts_core::formats::map::{MapManifest, MapTerrain, MapTile, TileGrid};
use opts_core::formats::scenery::sli::SplineProfile;

type Tiles = HashMap<(i32, i32), (MapTile, Option<MapTerrain>)>;

/// Reads every `tile_X_Y.map` of the map directory with its optional terrain file.
fn load_tiles(dir: &std::path::Path) -> Tiles {
    let mut tiles: Tiles = HashMap::new();
    for e in std::fs::read_dir(dir).expect("dir").flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        let Some(key) = name
            .strip_prefix("tile_")
            .and_then(|s| s.strip_suffix(".map"))
            .and_then(|s| s.split_once('_'))
            .and_then(|(x, y)| Some((x.parse().ok()?, y.parse().ok()?)))
        else {
            continue;
        };
        let Ok(tile) = MapTile::parse(&std::fs::read(e.path()).expect("tile")) else {
            continue;
        };
        let terrain = std::fs::read(dir.join(format!("{name}.terrain")))
            .ok()
            .and_then(|b| MapTerrain::parse(&b).ok());
        tiles.insert(key, (tile, terrain));
    }
    tiles
}

/// Measures the map's tile grid from the splines of all tiles.
fn measure_grid(dir: &std::path::Path, tiles: &Tiles) -> TileGrid {
    let world_coordinates = std::fs::read(dir.join("global.cfg"))
        .ok()
        .and_then(|b| MapManifest::parse(&b).ok())
        .is_some_and(|m| m.world_coordinates);
    let sample: Vec<_> = tiles
        .iter()
        .map(|(k, (t, _))| (*k, t.splines.clone()))
        .collect();
    TileGrid::measure(&sample, world_coordinates).0
}

/// The terrain each tile is drawn with: carved under the surfaces of its own and its eight
/// neighbours' splines, as the game does.
fn carve_all(
    tiles: &Tiles,
    grid: &TileGrid,
    profile: &mut impl FnMut(&str) -> Option<SplineProfile>,
) -> HashMap<(i32, i32), TileTerrain> {
    let mut carved: HashMap<(i32, i32), TileTerrain> = HashMap::new();
    let (mut lowered, mut guarded) = (0, 0);
    for (key, (_, terrain)) in tiles {
        let Some(t) = terrain else { continue };
        let mut surfaces = Vec::new();
        for dx in -1..=1 {
            for dy in -1..=1 {
                let other = (key.0 + dx, key.1 + dy);
                let Some((n, _)) = tiles.get(&other) else {
                    continue;
                };
                let o = grid.neighbour_offset(*key, other);
                for s in &n.splines {
                    if let Some(p) = profile(&s.sli_file) {
                        surfaces.extend(spline_ground_triangles(&p, s, (o.x, o.y)));
                    }
                }
            }
        }
        let (c, stats) = carve_terrain(
            &TileTerrain::new(t.clone(), grid.cell(key.1) as f32),
            &surfaces,
        );
        lowered += stats.lowered;
        guarded += stats.guarded;
        carved.insert(*key, c);
    }
    println!("grid {grid:?}; vertices lowered {lowered}, left by the guard {guarded}");
    carved
}

type Tally<'a> = BTreeMap<(&'a str, &'a str), (u64, u64)>;

fn print_report(
    n_tiles: usize,
    drawn: usize,
    counts: (&Tally, &BTreeMap<&str, usize>),
    worst: &mut [(u64, String, (i32, i32), u64)],
) {
    let (tally, bad_splines) = counts;
    println!("tiles {n_tiles}, drawn splines {drawn}");
    for ((fp, own), (n, c)) in tally {
        println!(
            "{fp:13} {own:15} sampled {n:8} m  covered {c:7} m ({:.2} %)",
            100.0 * *c as f64 / (*n).max(1) as f64
        );
    }
    println!("splines with >= 2 m covered: {bad_splines:?}");
    worst.sort_by_key(|w| std::cmp::Reverse(w.0));
    for (c, sli, key, id) in worst.iter().take(15) {
        println!("  {c:5} m  tile {key:?} id {id}  {sli}");
    }
}

/// Centre surface height: the profile point nearest the centreline.
fn centre_surface(p: &SplineProfile) -> f32 {
    p.graphical_profiles
        .iter()
        .flat_map(|g| &g.points)
        .min_by(|a, b| a.x.abs().total_cmp(&b.x.abs()))
        .map_or(0.0, |q| q.z)
}

fn main() {
    let dir = std::path::PathBuf::from(std::env::args().nth(1).expect("map dir"));
    let root = OmsiInstall::root_from_map_dir(&dir).expect("OMSI root");
    let install = OmsiInstall::new(&root);

    let tiles = load_tiles(&dir);
    let mut profiles: HashMap<String, Option<SplineProfile>> = HashMap::new();
    let mut profile = |sli: &str| {
        profiles
            .entry(sli.to_lowercase())
            .or_insert_with(|| {
                let p = install.find_file(sli)?;
                SplineProfile::parse(&std::fs::read(p).ok()?).ok()
            })
            .clone()
    };
    let grid = measure_grid(&dir, &tiles);
    let carved = carve_all(&tiles, &grid, &mut profile);

    // (surface?, own tile?) -> (metres sampled, metres covered); splines with >= 2 m covered.
    let mut tally: BTreeMap<(&str, &str), (u64, u64)> = BTreeMap::new();
    let mut bad_splines: BTreeMap<&str, usize> = BTreeMap::new();
    let mut worst: Vec<(u64, String, (i32, i32), u64)> = Vec::new();
    let mut drawn = 0usize;
    for (key, (tile, _)) in &tiles {
        for s in &tile.splines {
            let Some(p) = profile(&s.sli_file) else {
                continue;
            };
            // Tunnels run under the terrain on purpose.
            if p.only_editor
                || p.flat_width() < 3.0
                || s.sli_file.to_lowercase().contains("tunnel")
                || extrude_spline_segment(&p, s, (0.0, 0.0)).is_empty()
            {
                continue;
            }
            drawn += 1;
            let fp = if p.ground_surface().is_empty() {
                "no surface"
            } else {
                "surface"
            };
            // Centre surface: the profile point nearest the centreline.
            let centre_z = centre_surface(&p);
            let len = s.length_m.max(0.01);
            let steps = len.ceil() as usize;
            let mut covered_here = 0u64;
            for k in 0..=steps {
                let pose = spline_pose(s, len * k as f32 / steps.max(1) as f32);
                let (host, local) = grid.tile_at(
                    bevy::math::DVec2::new(f64::from(pose.x), f64::from(pose.y)),
                    *key,
                );
                let Some(t) = carved.get(&host) else { continue };
                let (lx, ly) = (local.x as f32, local.y as f32);
                let own = if host == *key {
                    "own tile"
                } else {
                    "neighbour tile"
                };
                let e = tally.entry((fp, own)).or_default();
                e.0 += 1;
                if t.height(lx, ly) > pose.z + centre_z + 0.05 {
                    e.1 += 1;
                    covered_here += 1;
                }
            }
            if covered_here >= 2 {
                *bad_splines.entry(fp).or_default() += 1;
                worst.push((covered_here, s.sli_file.clone(), *key, s.id_code));
            }
        }
    }
    print_report(tiles.len(), drawn, (&tally, &bad_splines), &mut worst);
}
