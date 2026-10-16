//! How much of a map's drawn splines does the terrain cover? Rebuilds every tile's terrain as the
//! game does (`carve_terrain_under_roads` with the footprints of the tile's own splines) and walks
//! the centreline of every drawn spline in 1 m steps: a sample counts as covered when the carved
//! terrain of the tile it lies in is more than 5 cm above the profile's centre surface. Samples
//! are split by whether they lie in the spline's own tile or in a neighbour, and by whether the
//! spline has a terrain footprint (`road_footprint`).
//!
//! With `--neighbours` each tile is also carved under the footprints of its eight neighbours'
//! splines (`footprint_from_neighbour`), as the game does since the Neuenbreid fix.
//!
//! Usage: `cargo run --release -p opts-bevy --example spline_terrain_cover -- <map dir> [tile size] [--neighbours]`
use std::collections::{BTreeMap, HashMap};

use opts_bevy::install::OmsiInstall;
use opts_bevy::scenery::{extrude_spline_segment, spline_pose};
use opts_bevy::terrain::{
    carve_terrain_under_roads, footprint_from_neighbour, road_footprint, sample_terrain,
    set_tile_size,
};
use opts_core::formats::map::{MapTerrain, MapTile};
use opts_core::formats::scenery::sli::SplineProfile;

fn main() {
    let mut args = std::env::args().skip(1);
    let dir = std::path::PathBuf::from(args.next().expect("map dir"));
    let rest: Vec<String> = args.collect();
    let neighbours = rest.iter().any(|a| a == "--neighbours");
    let tile_size: f32 = rest.iter().find_map(|s| s.parse().ok()).unwrap_or(300.0);
    set_tile_size(tile_size);
    let root = OmsiInstall::root_from_map_dir(&dir).expect("OMSI root");
    let install = OmsiInstall::new(&root);

    let mut tiles: HashMap<(i32, i32), (MapTile, Option<MapTerrain>)> = HashMap::new();
    for e in std::fs::read_dir(&dir).expect("dir").flatten() {
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
    // The terrain each tile is drawn with.
    let mut carved: HashMap<(i32, i32), MapTerrain> = HashMap::new();
    for (key, (tile, terrain)) in &tiles {
        let Some(t) = terrain else { continue };
        let mut roads: Vec<_> = tile
            .splines
            .iter()
            .filter_map(|s| road_footprint(&profile(&s.sli_file)?, s))
            .collect();
        if neighbours {
            for dx in -1..=1 {
                for dy in -1..=1 {
                    let Some((n, _)) = tiles.get(&(key.0 + dx, key.1 + dy)) else {
                        continue;
                    };
                    if (dx, dy) == (0, 0) {
                        continue;
                    }
                    let offset = (dx as f32 * tile_size, dy as f32 * tile_size);
                    roads.extend(n.splines.iter().filter_map(|s| {
                        footprint_from_neighbour(
                            &road_footprint(&profile(&s.sli_file)?, s)?,
                            offset,
                        )
                    }));
                }
            }
        }
        carved.insert(*key, carve_terrain_under_roads(t, &roads));
    }

    // (footprint?, own tile?) -> (metres sampled, metres covered); splines with >= 2 m covered.
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
            let fp = if road_footprint(&p, s).is_some() {
                "footprint"
            } else {
                "no footprint"
            };
            // Centre surface: the profile point nearest the centreline.
            let centre_z = p
                .graphical_profiles
                .iter()
                .flat_map(|g| &g.points)
                .min_by(|a, b| a.x.abs().total_cmp(&b.x.abs()))
                .map_or(0.0, |q| q.z);
            let len = s.length_m.max(0.01);
            let steps = len.ceil() as usize;
            let mut covered_here = 0u64;
            for k in 0..=steps {
                let pose = spline_pose(s, len * k as f32 / steps.max(1) as f32);
                let (dx, dy) = (
                    (pose.x / tile_size).floor() as i32,
                    (pose.y / tile_size).floor() as i32,
                );
                let host = (key.0 + dx, key.1 + dy);
                let Some(t) = carved.get(&host) else { continue };
                let (lx, ly) = (
                    pose.x - dx as f32 * tile_size,
                    pose.y - dy as f32 * tile_size,
                );
                let own = if (dx, dy) == (0, 0) {
                    "own tile"
                } else {
                    "neighbour tile"
                };
                let e = tally.entry((fp, own)).or_default();
                e.0 += 1;
                if sample_terrain(t, lx, ly) > pose.z + centre_z + 0.05 {
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
    println!("tiles {}, drawn splines {drawn}", tiles.len());
    for ((fp, own), (n, c)) in &tally {
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
