//! For `[absheight]` objects: object z vs nearest spline end z / terrain (scratch investigation).
use opts_omsi::formats::map::{MapTerrain, MapTile};
use opts_omsi::formats::scenery::SceneryObject;
fn main() {
    let dir = std::path::PathBuf::from(std::env::args().nth(1).expect("map dir"));
    let root = dir.parent().unwrap().parent().unwrap().to_path_buf();
    let mut rows: Vec<(f32, String, f32, f32, f32)> = Vec::new();
    let mut cache: std::collections::HashMap<String, Option<bool>> = Default::default();
    for e in std::fs::read_dir(&dir).expect("dir").flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        if !name.starts_with("tile_") || !name.ends_with(".map") {
            continue;
        }
        let base = e.path().to_string_lossy().into_owned();
        let (Ok(b), Ok(tb)) = (
            std::fs::read(&base),
            std::fs::read(format!("{base}.terrain")),
        ) else {
            continue;
        };
        let (Ok(tile), Ok(t)) = (MapTile::parse(&b), MapTerrain::parse(&tb)) else {
            continue;
        };
        let dim = t.grid_cells as usize + 1;
        for o in &tile.objects {
            let abs = cache.entry(o.sco_file.clone()).or_insert_with(|| {
                let p = root.join(o.sco_file.replace('\\', "/"));
                let p = if p.exists() { p } else { return None };
                SceneryObject::parse(&std::fs::read(p).ok()?)
                    .ok()
                    .map(|s| s.abs_height)
            });
            if *abs != Some(true) {
                continue;
            }
            let (ix, iy) = (
                ((o.position.x / 5.0) as usize).min(dim - 2),
                ((o.position.y / 5.0) as usize).min(dim - 2),
            );
            let th = t.heights[iy * dim + ix];
            // nearest spline start/end z
            let mut best = (f32::MAX, 0.0);
            for s in &tile.splines {
                let d = (s.position.x - o.position.x).hypot(s.position.y - o.position.y);
                if d < best.0 {
                    best = (d, s.position.z);
                }
            }
            rows.push((
                o.position.z - best.1,
                o.sco_file.rsplit('\\').next().unwrap().to_string(),
                o.position.z,
                th,
                best.0,
            ));
        }
    }
    let mut off: std::collections::BTreeMap<String, (usize, f32, f32)> = Default::default();
    for r in &rows {
        let d = r.2 - r.3;
        if d.abs() > 0.3 {
            let e = off.entry(r.1.clone()).or_insert((0, f32::MAX, f32::MIN));
            e.0 += 1;
            e.1 = e.1.min(d);
            e.2 = e.2.max(d);
        }
    }
    for (k, v) in &off {
        println!(
            "offset from terrain: {k:35} x{} dz {:.2}..{:.2}",
            v.0, v.1, v.2
        );
    }
    rows.sort_by(|a, b| a.0.total_cmp(&b.0));
    println!("{} absheight objects", rows.len());
    let n = rows.len().max(1);
    for i in [0, n / 20, n / 4, n / 2, 3 * n / 4, 19 * n / 20, n - 1] {
        if let Some(r) = rows.get(i) {
            println!(
                "dz(obj-spline)={:7.2} {:30} z={:7.2} terrain={:7.2} splineDist={:6.1}",
                r.0, r.1, r.2, r.3, r.4
            );
        }
    }
}
