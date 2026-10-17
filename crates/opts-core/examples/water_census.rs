//! Census of `.map.water` files (M-0025 / M-0067): level classes, corner continuity across
//! neighbouring tiles and the relation to the terrain of the same tile.
//! Usage: `water_census "<OMSI 2>/maps"`.
use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use opts_core::formats::map::{MapTerrain, MapWater};

fn main() {
    let root = std::env::args().nth(1).expect("maps directory");
    let mut files: HashMap<(String, i32, i32), (MapWater, Option<MapTerrain>)> = HashMap::new();
    for map in std::fs::read_dir(&root).unwrap().flatten() {
        let Ok(rd) = std::fs::read_dir(map.path()) else { continue };
        for f in rd.flatten() {
            let name = f.file_name().to_string_lossy().to_lowercase();
            let Some(stem) = name.strip_suffix(".map.water") else { continue };
            let mut p = stem.strip_prefix("tile_").unwrap_or(stem).split('_');
            let (Some(x), Some(y)) = (p.next().and_then(|s| s.parse().ok()), p.next().and_then(|s| s.parse().ok())) else { continue };
            let Ok(w) = MapWater::parse(&std::fs::read(f.path()).unwrap()) else { continue };
            let t = Path::new(&f.path().to_string_lossy().trim_end_matches(".water").to_string()).with_extension("map.terrain");
            let terrain = std::fs::read(&t).ok().and_then(|b| MapTerrain::parse(&b).ok());
            files.insert((map.file_name().to_string_lossy().into_owned(), x, y), (w, terrain));
        }
    }
    println!("{} water files in {} maps", files.len(), files.keys().map(|k| &k.0).collect::<std::collections::HashSet<_>>().len());
    let varying = files.values().filter(|(w, _)| w.levels.iter().any(|l| (l - w.levels[0]).abs() > 1e-4)).count();
    println!("files whose four levels differ: {varying}");
    let mut consts: BTreeMap<i64, usize> = BTreeMap::new();
    for (w, _) in files.values() {
        if w.levels.iter().all(|l| (l - w.levels[0]).abs() < 1e-4) {
            *consts.entry((w.levels[0] * 100.0).round() as i64).or_default() += 1;
        }
    }
    let mut top: Vec<_> = consts.into_iter().collect();
    top.sort_by_key(|e| std::cmp::Reverse(e.1));
    println!("most common constant levels (m): {:?}", top.iter().take(8).map(|(l, c)| (*l as f32 / 100.0, *c)).collect::<Vec<_>>());
    // Corner continuity: does value j of the neighbour equal value i of this tile?
    for (label, (dx, dy)) in [("east neighbour", (1, 0)), ("north neighbour", (0, 1))] {
        let mut m = [[0usize; 4]; 4];
        let mut pairs = 0;
        for ((map, x, y), (a, _)) in &files {
            let Some((b, _)) = files.get(&(map.clone(), x + dx, y + dy)) else { continue };
            let flat = |w: &MapWater| w.levels.iter().all(|l| (l - w.levels[0]).abs() < 1e-4);
            if flat(a) && flat(b) {
                continue;
            }
            pairs += 1;
            for i in 0..4 {
                for j in 0..4 {
                    m[i][j] += usize::from((a.levels[i] - b.levels[j]).abs() < 1e-4);
                }
            }
        }
        println!("{label}: {pairs} pairs with varying levels; matches of this[i] == neighbour[j]:");
        for (i, row) in m.iter().enumerate() {
            println!("  i={i}: {row:?}");
        }
    }
    // Terrain below the water surface (bilinear corner levels) per level class.
    let mut classes: BTreeMap<&str, (usize, usize)> = BTreeMap::new();
    for (w, t) in files.values() {
        let Some(t) = t else { continue };
        let n = t.grid_cells as usize;
        let dim = n + 1;
        let wet = (0..dim * dim).any(|k| {
            let (i, j) = ((k % dim) as f32 / n as f32, (k / dim) as f32 / n as f32);
            let lvl = (1.0 - i) * (1.0 - j) * w.levels[0] + i * (1.0 - j) * w.levels[1] + (1.0 - i) * j * w.levels[2] + i * j * w.levels[3];
            t.heights[k] < lvl
        });
        let c = if w.levels.iter().all(|l| (l + 5.0).abs() < 1e-4) { "default -5" } else if w.levels.iter().all(|l| (l - w.levels[0]).abs() < 1e-4) { "constant other" } else { "varying" };
        let e = classes.entry(c).or_default();
        e.0 += 1;
        e.1 += usize::from(wet);
    }
    for (c, (n, wet)) in classes {
        println!("{c}: {n} tiles, {wet} have terrain below the water surface");
    }
}
