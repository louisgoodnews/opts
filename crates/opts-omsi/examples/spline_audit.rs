//! Checks whether placed splines connect: for each spline that names another spline in its two
//! connection fields, measures the endpoint gap under the current geometry model.
//! Usage: `cargo run --release -p opts-core --example spline_audit -- <map dir>`.
#![allow(dead_code)]
use std::collections::HashMap;

use opts_omsi::formats::map::{MapManifest, MapTile, TileGrid};

#[derive(Clone)]
struct Seg {
    id: u64,
    c1: u64,
    c2: u64,
    start: [f64; 3],
    end: [f64; 3],
    h0: f64,
    h1: f64,
    extra: Vec<String>,
    sli: String,
    len: f32,
    radius: f32,
    has_h: bool,
    flag: i32,
}

fn pose(inst: &opts_omsi::formats::map::MapSplineInstance, ox: f64, oy: f64) -> Seg {
    let phi = f64::from(inst.heading_deg).to_radians();
    let (x0, y0) = (
        ox + f64::from(inst.position.x),
        oy + f64::from(inst.position.y),
    );
    let len = f64::from(inst.length_m);
    let r = f64::from(inst.radius_m);
    let (h1, x1, y1) = if r.abs() > 1e-3 {
        let h = phi + len / r;
        (
            h,
            x0 - r * (h.cos() - phi.cos()),
            y0 + r * (h.sin() - phi.sin()),
        )
    } else {
        (phi, x0 + len * phi.sin(), y0 + len * phi.cos())
    };
    let gs = f64::from(inst.gradient_start);
    let ge = f64::from(inst.gradient_end);
    let z1 = f64::from(inst.position.z) + gs * len + (ge - gs) * len / 2.0;
    Seg {
        id: inst.id_code,
        c1: inst.conn_id_1,
        c2: inst.conn_id_2,
        start: [x0, y0, f64::from(inst.position.z)],
        end: [x1, y1, z1],
        h0: phi,
        h1,
        extra: inst.raw_params.clone(),
        sli: inst.sli_file.clone(),
        len: inst.length_m,
        radius: inst.radius_m,
        has_h: inst.has_height_profile,
        flag: inst.flag,
    }
}

fn main() {
    let dir = std::env::args().nth(1).expect("map dir");
    let mut tiles: Vec<((i32, i32), MapTile)> = Vec::new();
    for e in std::fs::read_dir(&dir).expect("dir").flatten() {
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
        let Ok(bytes) = std::fs::read(e.path()) else {
            continue;
        };
        let Ok(tile) = MapTile::parse(&bytes) else {
            continue;
        };
        tiles.push(((tx, ty), tile));
    }
    // The tiles' grid: 300 m, or the Mercator rows of a `[worldcoordinates]` map.
    let world_coordinates = std::fs::read(std::path::Path::new(&dir).join("global.cfg"))
        .ok()
        .and_then(|b| MapManifest::parse(&b).ok())
        .is_some_and(|m| m.world_coordinates);
    let sample: Vec<_> = tiles.iter().map(|(k, t)| (*k, t.splines.clone())).collect();
    let (grid, _) = TileGrid::measure(&sample, world_coordinates);
    let mut segs: Vec<Seg> = Vec::new();
    for (key, tile) in &tiles {
        let o = grid.origin(*key, (0, 0));
        for s in &tile.splines {
            segs.push(pose(s, o.x, o.y));
        }
    }
    let by_id: HashMap<u64, usize> = segs.iter().enumerate().map(|(i, s)| (s.id, i)).collect();
    println!(
        "splines: {}, with c1: {}, with c2: {}, c1 resolvable: {}, c2 resolvable: {}",
        segs.len(),
        segs.iter().filter(|s| s.c1 != 0).count(),
        segs.iter().filter(|s| s.c2 != 0).count(),
        segs.iter()
            .filter(|s| s.c1 != 0 && by_id.contains_key(&s.c1))
            .count(),
        segs.iter()
            .filter(|s| s.c2 != 0 && by_id.contains_key(&s.c2))
            .count()
    );
    // Vertical audit: for A.end -> B.start pairs (planar gap <= 5 cm) compare dz to candidate models.
    let mut rows = Vec::new();
    let mut hflags = Vec::new();
    for a in &segs {
        for cid in [a.c1, a.c2] {
            let Some(&j) = by_id.get(&cid) else { continue };
            let b = &segs[j];
            let d = ((a.end[0] - b.start[0]).powi(2) + (a.end[1] - b.start[1]).powi(2)).sqrt();
            if d < 0.05 {
                let f = |k: usize| {
                    a.extra
                        .get(k)
                        .and_then(|x| x.parse::<f64>().ok())
                        .unwrap_or(0.0)
                };
                hflags.push((a.has_h, a.flag));
                rows.push((
                    b.start[2] - a.start[2],
                    f64::from(a.len),
                    f(4),
                    f(5),
                    f(0),
                    f(1),
                    f(2),
                    f(3),
                    f(6),
                    if a.has_h { 1.0 } else { 0.0 },
                ));
            }
        }
    }
    println!("A.end->B.start pairs: {}", rows.len());
    type Model = (
        &'static str,
        fn(&(f64, f64, f64, f64, f64, f64, f64, f64, f64, f64)) -> f64,
    );
    let models: Vec<Model> = vec![
        ("zero", |_| 0.0),
        ("len*(p15+p16)/2", |r| r.1 * (r.2 + r.3) / 2.0),
        ("len*p16", |r| r.1 * r.3),
        ("len*p15", |r| r.1 * r.2),
        ("len*(p15+p16)/200 (percent)", |r| r.1 * (r.2 + r.3) / 200.0),
        ("len*tan(avg deg)", |r| {
            r.1 * (((r.2 + r.3) / 2.0).to_radians()).tan()
        }),
        ("len*sin(avg deg)", |r| {
            r.1 * (((r.2 + r.3) / 2.0).to_radians()).sin()
        }),
        ("len*(p15+p16)/2000 (permille)", |r| {
            r.1 * (r.2 + r.3) / 2000.0
        }),
        ("p17", |r| r.8),
        ("len*(p11+p12)/2", |r| r.1 * (r.4 + r.5) / 2.0),
        ("len*(p11+p12)/200 (percent)", |r| r.1 * (r.4 + r.5) / 200.0),
        ("p13 if set else len*(p11+p12)/200", |r| {
            if r.6.abs() > 1e-9 {
                r.6
            } else {
                r.1 * (r.4 + r.5) / 200.0
            }
        }),
        ("len*(p11+p12)/200 + len*(p15+p16)/2", |r| {
            r.1 * (r.4 + r.5) / 200.0 + r.1 * (r.2 + r.3) / 2.0
        }),
    ];
    for (name, f) in &models {
        let ok = rows.iter().filter(|r| (r.0 - f(r)).abs() <= 0.05).count();
        println!(
            "  dz model {name:34}: within 5 cm for {:5.1}%",
            100.0 * ok as f64 / rows.len().max(1) as f64
        );
    }
    for h in [0.0, 1.0] {
        let sel: Vec<_> = rows.iter().filter(|r| r.9 == h).collect();
        let grad =
            |r: &&(f64, f64, f64, f64, f64, f64, f64, f64, f64, f64)| r.1 * (r.4 + r.5) / 200.0;
        let g_ok = sel.iter().filter(|r| (r.0 - grad(r)).abs() <= 0.05).count();
        let p13_only = sel
            .iter()
            .filter(|r| r.4 == 0.0 && r.5 == 0.0 && r.6 != 0.0 && r.7 == 0.0)
            .count();
        let p13_only_ok = sel
            .iter()
            .filter(|r| {
                r.4 == 0.0 && r.5 == 0.0 && r.6 != 0.0 && r.7 == 0.0 && (r.0 - r.6).abs() <= 0.05
            })
            .count();
        let both = sel.iter().filter(|r| r.6 != 0.0 && r.7 != 0.0).count();
        let both_flat = sel
            .iter()
            .filter(|r| r.6 != 0.0 && r.7 != 0.0 && (r.0 - grad(r)).abs() <= 0.05)
            .count();
        println!("  has_h={h}: pairs {} gradient-model ok {g_ok}; only p13 set: {p13_only} (dz==p13: {p13_only_ok}); p13&p14 both set: {both} (gradient model ok: {both_flat})", sel.len());
    }
}
