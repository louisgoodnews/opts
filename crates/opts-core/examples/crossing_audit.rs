//! Where do road spline ends meet the rest of the network? For every spline end that touches no
//! other spline, test whether it meets a path end of a placed scenery object (a crossing), under
//! both rotation senses. Usage: `-- <map dir>`.
#![allow(clippy::needless_range_loop)]
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use opts_core::formats::map::MapTile;
use opts_core::formats::scenery::SceneryObject;

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
type Pt = (f64, f64);

fn main() {
    let map_dir = PathBuf::from(std::env::args().nth(1).expect("map dir"));
    let root = map_dir
        .parent()
        .and_then(Path::parent)
        .expect("root")
        .to_path_buf();
    let mut tiles = Vec::new();
    for e in std::fs::read_dir(&map_dir).expect("dir").flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        let Some(rest) = name
            .strip_prefix("tile_")
            .and_then(|r| r.strip_suffix(".map"))
        else {
            continue;
        };
        let mut it = rest.split('_');
        let (Some(Ok(tx)), Some(Ok(ty))) = (
            it.next().map(str::parse::<i32>),
            it.next().map(str::parse::<i32>),
        ) else {
            continue;
        };
        if let Some(t) = std::fs::read(e.path())
            .ok()
            .and_then(|b| MapTile::parse(&b).ok())
        {
            tiles.push((tx, ty, t));
        }
    }
    // spline ends: (spline index, point)
    let mut ends: Vec<(usize, Pt, bool)> = Vec::new(); // bool: is_road (has traffic path) approximated by sli name
    let mut idx = 0;
    for (tx, ty, t) in &tiles {
        for s in &t.splines {
            let phi = f64::from(s.heading_deg).to_radians();
            let (x0, y0) = (
                f64::from(*tx) * 300.0 + f64::from(s.position.x),
                f64::from(*ty) * 300.0 + f64::from(s.position.y),
            );
            let (len, r) = (f64::from(s.length_m), f64::from(s.radius_m));
            let (x1, y1) = if r.abs() > 1e-3 {
                let h = phi + len / r;
                (
                    x0 - r * (h.cos() - phi.cos()),
                    y0 + r * (h.sin() - phi.sin()),
                )
            } else {
                (x0 + len * phi.sin(), y0 + len * phi.cos())
            };
            let n = s.sli_file.to_lowercase();
            let road = !(n.contains("bordstein")
                || n.contains("wand")
                || n.contains("zaun")
                || n.contains("gras")
                || n.contains("ter_")
                || n.contains("bre_b"));
            ends.push((idx, (x0, y0), road));
            ends.push((idx, (x1, y1), road));
            idx += 1;
        }
    }
    let cell = |p: Pt| ((p.0 / 2.0).floor() as i64, (p.1 / 2.0).floor() as i64);
    let mut grid: HashMap<(i64, i64), Vec<usize>> = HashMap::new();
    for (i, e) in ends.iter().enumerate() {
        grid.entry(cell(e.1)).or_default().push(i);
    }
    let joined = |i: usize, tol: f64| -> bool {
        let (c, p) = (cell(ends[i].1), ends[i].1);
        for dx in -1..=1 {
            for dy in -1..=1 {
                if let Some(v) = grid.get(&(c.0 + dx, c.1 + dy)) {
                    for &j in v {
                        if ends[j].0 != ends[i].0
                            && (ends[j].1 .0 - p.0).hypot(ends[j].1 .1 - p.1) <= tol
                        {
                            return true;
                        }
                    }
                }
            }
        }
        false
    };
    // object path ends
    let mut objs: Vec<Vec<Pt>> = vec![Vec::new(), Vec::new()]; // [cw, ccw]
    let mut cache: HashMap<String, Option<SceneryObject>> = HashMap::new();
    for (tx, ty, t) in &tiles {
        for o in &t.objects {
            let sco = cache.entry(o.sco_file.clone()).or_insert_with(|| {
                find_ci(&root, &o.sco_file)
                    .and_then(|p| std::fs::read(p).ok())
                    .and_then(|b| SceneryObject::parse(&b).ok())
            });
            let Some(sco) = sco else { continue };
            let (ox, oy) = (
                f64::from(*tx) * 300.0 + f64::from(o.position.x),
                f64::from(*ty) * 300.0 + f64::from(o.position.y),
            );
            let th = f64::from(o.heading_deg).to_radians();
            for p in sco.paths.iter().filter(|p| p.path_type == 0) {
                let (sx, sy) = (f64::from(p.start.x), f64::from(p.start.y));
                let ph = f64::from(p.heading_deg).to_radians();
                let (l, rr) = (f64::from(p.length_m), f64::from(p.radius_m));
                let (ex, ey) = if rr.abs() > 1e-3 && rr.abs() < 1.0e5 {
                    let h = ph + l / rr;
                    (
                        sx - rr * (h.cos() - ph.cos()),
                        sy + rr * (h.sin() - ph.sin()),
                    )
                } else {
                    (sx + l * ph.sin(), sy + l * ph.cos())
                };
                for (lx, ly) in [(sx, sy), (ex, ey)] {
                    let (c, s) = (th.cos(), th.sin());
                    objs[0].push((ox + lx * c + ly * s, oy - lx * s + ly * c));
                    objs[1].push((ox + lx * c - ly * s, oy + lx * s + ly * c));
                }
            }
        }
    }
    let mut ogrid: [HashMap<(i64, i64), Vec<Pt>>; 2] = [HashMap::new(), HashMap::new()];
    for k in 0..2 {
        for p in &objs[k] {
            ogrid[k].entry(cell(*p)).or_default().push(*p);
        }
    }
    let near_obj = |k: usize, p: Pt, tol: f64| -> bool {
        let c = cell(p);
        for dx in -1..=1 {
            for dy in -1..=1 {
                if let Some(v) = ogrid[k].get(&(c.0 + dx, c.1 + dy)) {
                    if v.iter().any(|q| (q.0 - p.0).hypot(q.1 - p.1) <= tol) {
                        return true;
                    }
                }
            }
        }
        false
    };
    let (mut total, mut j_tight, mut j_loose, mut free_road, mut at_obj_cw, mut at_obj_ccw) =
        (0, 0, 0, 0, 0, 0);
    for i in 0..ends.len() {
        total += 1;
        if joined(i, 0.1) {
            j_tight += 1;
            continue;
        }
        if joined(i, 1.5) {
            j_loose += 1;
            continue;
        }
        if !ends[i].2 {
            continue;
        }
        free_road += 1;
        if near_obj(0, ends[i].1, 0.75) {
            at_obj_cw += 1;
        }
        if near_obj(1, ends[i].1, 0.75) {
            at_obj_ccw += 1;
        }
    }
    println!("spline ends: {total}; joined to another spline (<=0.1 m): {j_tight}, nearly joined (0.1-1.5 m): {j_loose}");
    println!("free road-like ends: {free_road}; of which at an object path end: clockwise rotation {at_obj_cw}, counter-clockwise {at_obj_ccw} (object path ends: {})", objs[0].len());
}
