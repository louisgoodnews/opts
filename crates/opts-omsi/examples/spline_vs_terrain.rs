//! Compares spline start heights with the tile's terrain height beneath them (scratch investigation).
use opts_omsi::formats::map::{MapTerrain, MapTile};
fn main() {
    let dir = std::env::args().nth(1).expect("map dir");
    let mut rows: Vec<(f32, String)> = Vec::new();
    let mut ends: Vec<(f32, bool)> = Vec::new();
    for e in std::fs::read_dir(&dir).expect("dir").flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        let Some(rest) = name.strip_suffix(".map") else {
            continue;
        };
        if !rest.starts_with("tile_") {
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
        for s in &tile.splines {
            let (fx, fy) = (s.position.x / 5.0, s.position.y / 5.0);
            if fx < 0.0 || fy < 0.0 || fx >= (dim - 1) as f32 || fy >= (dim - 1) as f32 {
                continue;
            }
            let (ix, iy) = (fx as usize, fy as usize);
            let (ax, ay) = (fx - ix as f32, fy - iy as f32);
            let h = |x: usize, y: usize| t.heights[y * dim + x];
            let th = h(ix, iy) * (1.0 - ax) * (1.0 - ay)
                + h(ix + 1, iy) * ax * (1.0 - ay)
                + h(ix, iy + 1) * (1.0 - ax) * ay
                + h(ix + 1, iy + 1) * ax * ay;
            rows.push((s.position.z - th, s.sli_file.clone()));
            // End point under the plain/hermite height model.
            let phi = f64::from(s.heading_deg).to_radians();
            let (len, r) = (f64::from(s.length_m), f64::from(s.radius_m));
            let (ex, ey) = if r.abs() > 1e-3 {
                let h = phi + len / r;
                (
                    f64::from(s.position.x) - r * (h.cos() - phi.cos()),
                    f64::from(s.position.y) + r * (h.sin() - phi.sin()),
                )
            } else {
                (
                    f64::from(s.position.x) + len * phi.sin(),
                    f64::from(s.position.y) + len * phi.cos(),
                )
            };
            let (gx, gy) = (ex as f32 / 5.0, ey as f32 / 5.0);
            if gx < 0.0 || gy < 0.0 || gx >= (dim - 1) as f32 || gy >= (dim - 1) as f32 {
                continue;
            }
            let (jx, jy) = (gx as usize, gy as usize);
            let (bx, by) = (gx - jx as f32, gy - jy as f32);
            let eh = h(jx, jy) * (1.0 - bx) * (1.0 - by)
                + h(jx + 1, jy) * bx * (1.0 - by)
                + h(jx, jy + 1) * (1.0 - bx) * by
                + h(jx + 1, jy + 1) * bx * by;
            let (gs, ge) = (f64::from(s.gradient_start), f64::from(s.gradient_end));
            let dz_total = if s.has_height_profile
                && s.raw_params
                    .get(2)
                    .and_then(|v| v.parse::<f64>().ok())
                    .is_some_and(|v| v != 0.0)
            {
                s.raw_params[2].parse::<f64>().unwrap()
            } else {
                gs * len + (ge - gs) * len / 2.0
            };
            ends.push((s.position.z + dz_total as f32 - eh, s.has_height_profile));
        }
    }
    rows.sort_by(|a, b| a.0.total_cmp(&b.0));
    if std::env::var_os("BY_PROFILE").is_some() {
        let mut by: std::collections::BTreeMap<&str, Vec<f32>> = Default::default();
        for (dz, sli) in &rows {
            by.entry(sli.as_str()).or_default().push(*dz);
        }
        let mut list: Vec<_> = by.into_iter().collect();
        list.sort_by_key(|(_, v)| std::cmp::Reverse(v.len()));
        for (sli, mut v) in list.into_iter().take(40) {
            v.sort_by(f32::total_cmp);
            println!(
                "{:5} splines  median dz {:7.2}  p10 {:7.2}  p90 {:7.2}  {sli}",
                v.len(),
                v[v.len() / 2],
                v[v.len() / 10],
                v[v.len() * 9 / 10]
            );
        }
    }
    let n = rows.len();
    println!(
        "{n} splines; dz percentiles 5/25/50/75/95: {:?}",
        [n / 20, n / 4, n / 2, 3 * n / 4, 19 * n / 20].map(|i| rows[i].0)
    );
    let near = rows.iter().filter(|r| r.0.abs() < 0.3).count();
    println!(
        "within 30 cm of terrain: {near} ({:.1}%)",
        100.0 * near as f32 / n as f32
    );
    for flag in [false, true] {
        let mut v: Vec<f32> = ends.iter().filter(|e| e.1 == flag).map(|e| e.0).collect();
        v.sort_by(f32::total_cmp);
        let n = v.len().max(1);
        let near = v.iter().filter(|d| d.abs() < 0.3).count();
        println!(
            "end dz (spline_h={flag}): n={} within30cm={near} pct 5/25/50/75/95 {:?}",
            v.len(),
            [n / 20, n / 4, n / 2, 3 * n / 4, (19 * n / 20).min(n - 1)].map(|i| v.get(i).copied())
        );
    }
}
