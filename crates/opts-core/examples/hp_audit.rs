//! Is the terrain deformed to the `.sli` [heightprofile] strips (embankments) beside splines?
use opts_core::formats::map::{MapTerrain, MapTile};
use opts_core::formats::scenery::SplineProfile;

fn main() {
    let dir = std::path::PathBuf::from(std::env::args().nth(1).expect("map dir"));
    let root = dir.parent().unwrap().parent().unwrap().to_path_buf();
    let mut cache: std::collections::HashMap<String, Option<SplineProfile>> = Default::default();
    let (mut err_follow, mut err_flat, mut n) = (0.0f64, 0.0f64, 0usize);
    let mut buckets: std::collections::BTreeMap<i32, (f64, f64, usize)> = Default::default();
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
            if s.has_height_profile || s.length_m < 20.0 {
                continue;
            }
            let prof = cache.entry(s.sli_file.clone()).or_insert_with(|| {
                let p = root.join(s.sli_file.replace('\\', "/"));
                std::fs::read(p)
                    .ok()
                    .and_then(|b| SplineProfile::parse(&b).ok())
            });
            let Some(prof) = prof else { continue };
            if prof.height_profiles.is_empty() {
                continue;
            }
            let phi = s.heading_deg.to_radians();
            let d = s.length_m * 0.5;
            let r = s.radius_m;
            let (cx, cy, h) = if r.abs() > 1e-3 {
                let h = phi + d / r;
                (
                    s.position.x - r * (h.cos() - phi.cos()),
                    s.position.y + r * (h.sin() - phi.sin()),
                    h,
                )
            } else {
                (
                    s.position.x + d * phi.sin(),
                    s.position.y + d * phi.cos(),
                    phi,
                )
            };
            let zc = s.position.z
                + s.gradient_start * d
                + (s.gradient_end - s.gradient_start) * d * d / (2.0 * s.length_m);
            for hp in &prof.height_profiles {
                for k in 0..=10 {
                    let x = hp.x_start + (hp.x_end - hp.x_start) * k as f32 / 10.0;
                    let expect = zc + hp.z_start + (hp.z_end - hp.z_start) * k as f32 / 10.0;
                    let (px, py) = (cx + h.cos() * x, cy - h.sin() * x);
                    if px < 5.0 || py < 5.0 || px > 295.0 || py > 295.0 {
                        continue;
                    }
                    let (gx, gy) = ((px / 5.0) as usize, (py / 5.0) as usize);
                    let (fx, fy) = (px / 5.0 - gx as f32, py / 5.0 - gy as f32);
                    let hh = |i: usize, j: usize| t.heights[j * dim + i];
                    let th = hh(gx, gy) * (1.0 - fx) * (1.0 - fy)
                        + hh(gx + 1, gy) * fx * (1.0 - fy)
                        + hh(gx, gy + 1) * (1.0 - fx) * fy
                        + hh(gx + 1, gy + 1) * fx * fy;
                    let ef = f64::from((th - expect).abs());
                    let ec = f64::from((th - zc).abs()); // "flat at road level"
                    err_follow += ef;
                    err_flat += ec;
                    n += 1;
                    let bucket = (x.abs() / 5.0) as i32 * 5;
                    let e = buckets.entry(bucket).or_default();
                    e.0 += ef;
                    e.1 += ec;
                    e.2 += 1;
                }
            }
        }
    }
    println!(
        "{n} samples: mean |terrain - profile| {:.3}, mean |terrain - road z| {:.3}",
        err_follow / n as f64,
        err_flat / n as f64
    );
    for (b, (f, c, k)) in buckets {
        println!(
            "  |x| {b:>3}-{:>3} m: follow {:.2} flat-at-road {:.2} ({k})",
            b + 5,
            f / k as f64,
            c / k as f64
        );
    }
}
