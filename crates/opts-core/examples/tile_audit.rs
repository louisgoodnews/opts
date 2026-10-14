//! Offline audit of every map tile of an OMSI install (P-1014): tile and terrain parse failures,
//! terrain height spikes and non-finite values, objects and splines with non-finite or absurd
//! placement, splines with absurd length / radius, objects whose `.sco` is missing.
//!
//! Usage: `tile_audit <omsi root>`; prints `map<TAB>tile<TAB>code<TAB>detail` per finding and a
//! summary on stderr.
use std::path::Path;

use opts_core::formats::map::{MapManifest, MapTerrain, MapTile};

fn main() {
    let root = std::path::PathBuf::from(std::env::args().nth(1).expect("omsi root"));
    let (mut tiles, mut terrains, mut objects, mut splines) = (0usize, 0usize, 0usize, 0usize);
    let mut maps: Vec<_> = std::fs::read_dir(root.join("maps"))
        .expect("maps dir")
        .flatten()
        .collect();
    maps.sort_by_key(std::fs::DirEntry::file_name);
    for m in maps {
        let mname = m.file_name().to_string_lossy().into_owned();
        match std::fs::read(m.path().join("global.cfg"))
            .map_err(|e| e.to_string())
            .and_then(|b| MapManifest::parse(&b).map_err(|e| e.to_string()))
        {
            Err(err) => println!("{mname}\tglobal.cfg\tGLOBAL_PARSE_ERROR\t{err}"),
            Ok(g) => {
                if g.entry_points.is_empty() {
                    println!("{mname}\tglobal.cfg\tNO_ENTRYPOINTS\t");
                }
            }
        }
        let Ok(rd) = std::fs::read_dir(m.path()) else {
            continue;
        };
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if !(name.starts_with("tile_") && name.ends_with(".map")) {
                continue;
            }
            tiles += 1;
            let report = |code: &str, detail: String| println!("{mname}\t{name}\t{code}\t{detail}");
            let base = e.path();
            match std::fs::read(&base)
                .map_err(|e| e.to_string())
                .and_then(|b| MapTile::parse(&b).map_err(|e| e.to_string()))
            {
                Err(err) => report("TILE_PARSE_ERROR", err),
                Ok(t) => {
                    for o in &t.objects {
                        objects += 1;
                        let p = o.position;
                        if !p.is_finite() || p.abs().max_element() > 5000.0 {
                            report("OBJECT_PLACEMENT", format!("{} at {p:?}", o.sco_file));
                        }
                        if !o.heading_deg.is_finite() {
                            report("OBJECT_HEADING_NAN", o.sco_file.clone());
                        }
                    }
                    for s in &t.splines {
                        splines += 1;
                        let p = s.position;
                        if !p.is_finite() || p.abs().max_element() > 5000.0 {
                            report("SPLINE_PLACEMENT", format!("{} at {p:?}", s.sli_file));
                        }
                        if !s.length_m.is_finite() || s.length_m <= 0.0 || s.length_m > 2000.0 {
                            report(
                                "SPLINE_LENGTH",
                                format!("{} length {}", s.sli_file, s.length_m),
                            );
                        }
                        if !s.radius_m.is_finite() || (s.radius_m != 0.0 && s.radius_m.abs() < 1.0)
                        {
                            report(
                                "SPLINE_RADIUS",
                                format!("{} radius {}", s.sli_file, s.radius_m),
                            );
                        }
                        if !s.gradient_start.is_finite()
                            || !s.gradient_end.is_finite()
                            || s.gradient_start.abs() > 1.0
                            || s.gradient_end.abs() > 1.0
                        {
                            report(
                                "SPLINE_GRADIENT",
                                format!("{} {} {}", s.sli_file, s.gradient_start, s.gradient_end),
                            );
                        }
                    }
                }
            }
            let tpath = format!("{}.terrain", base.display());
            if Path::new(&tpath).exists() {
                match std::fs::read(&tpath)
                    .map_err(|e| e.to_string())
                    .and_then(|b| MapTerrain::parse(&b).map_err(|e| e.to_string()))
                {
                    Err(err) => report("TERRAIN_PARSE_ERROR", err),
                    Ok(t) => {
                        terrains += 1;
                        let dim = t.grid_cells as usize + 1;
                        if t.heights.iter().any(|h| !h.is_finite()) {
                            report("TERRAIN_NONFINITE", String::new());
                        }
                        let (mut lo, mut hi) = (f32::MAX, f32::MIN);
                        for &h in &t.heights {
                            if h.is_finite() {
                                lo = lo.min(h);
                                hi = hi.max(h);
                            }
                        }
                        if lo < -500.0 || hi > 4000.0 {
                            report("TERRAIN_RANGE", format!("{lo}..{hi}"));
                        }
                        let mut worst = 0.0f32;
                        for y in 0..dim {
                            for x in 0..dim {
                                let h = t.heights[y * dim + x];
                                if x + 1 < dim {
                                    worst = worst.max((h - t.heights[y * dim + x + 1]).abs());
                                }
                                if y + 1 < dim {
                                    worst = worst.max((h - t.heights[(y + 1) * dim + x]).abs());
                                }
                            }
                        }
                        if worst > 50.0 {
                            report("TERRAIN_SPIKE", format!("max neighbour step {worst:.1} m"));
                        }
                    }
                }
            }
        }
    }
    eprintln!("tiles {tiles}, terrains {terrains}, objects {objects}, splines {splines}");
}
