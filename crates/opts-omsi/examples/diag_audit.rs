//! Which terrain triangulation matches spline heights best? Samples spline centrelines at 1 m and compares
//! the spline elevation with bilinear and both triangle-diagonal interpolations of the terrain grid.
use opts_omsi::formats::map::{MapTerrain, MapTile};

fn tri(h: &[f32], dim: usize, x: f32, y: f32, diag_a: bool) -> f32 {
    let (gx, gy) = (
        (x / 5.0).clamp(0.0, (dim - 1) as f32 - 1e-4),
        (y / 5.0).clamp(0.0, (dim - 1) as f32 - 1e-4),
    );
    let (ix, iy) = (gx as usize, gy as usize);
    let (fx, fy) = (gx - ix as f32, gy - iy as f32);
    let (h00, h10, h01, h11) = (
        h[iy * dim + ix],
        h[iy * dim + ix + 1],
        h[(iy + 1) * dim + ix],
        h[(iy + 1) * dim + ix + 1],
    );
    if diag_a {
        // diagonal from (1,0) to (0,1)
        if fx + fy <= 1.0 {
            h00 + (h10 - h00) * fx + (h01 - h00) * fy
        } else {
            h11 + (h01 - h11) * (1.0 - fx) + (h10 - h11) * (1.0 - fy)
        }
    } else {
        // diagonal from (0,0) to (1,1)
        if fx >= fy {
            h00 + (h10 - h00) * fx + (h11 - h10) * fy
        } else {
            h00 + (h11 - h01) * fx + (h01 - h00) * fy
        }
    }
}

fn main() {
    let dir = std::env::args().nth(1).expect("map dir");
    let (mut e_bi, mut e_a, mut e_b, mut n) = (0.0f64, 0.0f64, 0.0f64, 0usize);
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
        for s in &tile.splines {
            if s.has_height_profile {
                continue;
            }
            let len = s.length_m;
            let mut d = 0.0;
            while d <= len {
                let phi = s.heading_deg.to_radians();
                let (x, y) = if s.radius_m.abs() > 1e-3 {
                    let r = s.radius_m;
                    let h = phi + d / r;
                    (
                        s.position.x - r * (h.cos() - phi.cos()),
                        s.position.y + r * (h.sin() - phi.sin()),
                    )
                } else {
                    (s.position.x + d * phi.sin(), s.position.y + d * phi.cos())
                };
                let z = s.position.z
                    + s.gradient_start * d
                    + (s.gradient_end - s.gradient_start) * d * d / (2.0 * len);
                if x > 5.0 && y > 5.0 && x < 295.0 && y < 295.0 {
                    let (gx, gy) = ((x / 5.0) as usize, (y / 5.0) as usize);
                    let (fx, fy) = (x / 5.0 - gx as f32, y / 5.0 - gy as f32);
                    let hh = |i: usize, j: usize| t.heights[j * dim + i];
                    let bi = hh(gx, gy) * (1.0 - fx) * (1.0 - fy)
                        + hh(gx + 1, gy) * fx * (1.0 - fy)
                        + hh(gx, gy + 1) * (1.0 - fx) * fy
                        + hh(gx + 1, gy + 1) * fx * fy;
                    // only count samples where the terrain is within 0.5 m of the road (ground-level roads)
                    if (z - bi).abs() < 0.5 {
                        e_bi += f64::from((z - bi).abs());
                        e_a += f64::from((z - tri(&t.heights, dim, x, y, true)).abs());
                        e_b += f64::from((z - tri(&t.heights, dim, x, y, false)).abs());
                        n += 1;
                    }
                }
                d += 1.0;
            }
        }
    }
    println!(
        "{n} samples; mean |dz| bilinear {:.4} diag(1,0)-(0,1) {:.4} diag(0,0)-(1,1) {:.4}",
        e_bi / n as f64,
        e_a / n as f64,
        e_b / n as f64
    );
}
