//! Lists the splines and objects of a tile near a point: `map_probe <map dir> <tx> <ty> <x> <y> [radius]`.
use opts_core::formats::map::{MapTerrain, MapTile};

fn main() {
    let mut a = std::env::args().skip(1);
    let dir = a.next().expect("map dir");
    let (tx, ty): (i32, i32) = (
        a.next().unwrap().parse().unwrap(),
        a.next().unwrap().parse().unwrap(),
    );
    let (x, y): (f32, f32) = (
        a.next().unwrap().parse().unwrap(),
        a.next().unwrap().parse().unwrap(),
    );
    let r: f32 = a.next().and_then(|v| v.parse().ok()).unwrap_or(40.0);
    let base = format!("{dir}/tile_{tx}_{ty}.map");
    let tile = MapTile::parse(&std::fs::read(&base).expect("tile")).expect("parse tile");
    let terrain = MapTerrain::parse(&std::fs::read(format!("{base}.terrain")).expect("terrain"))
        .expect("parse terrain");
    let dim = terrain.grid_cells as usize + 1;
    let th = |px: f32, py: f32| {
        let (fx, fy) = ((px / 5.0).clamp(0.0, 59.99), (py / 5.0).clamp(0.0, 59.99));
        let (ix, iy) = (fx as usize, fy as usize);
        let (ax, ay) = (fx - ix as f32, fy - iy as f32);
        let h = |i: usize, j: usize| terrain.heights[j * dim + i];
        (h(ix, iy) * (1.0 - ax) + h(ix + 1, iy) * ax) * (1.0 - ay)
            + (h(ix, iy + 1) * (1.0 - ax) + h(ix + 1, iy + 1) * ax) * ay
    };
    println!("terrain at point: {:.2}", th(x, y));
    for s in &tile.splines {
        // Sample the centreline for the nearest approach.
        let phi = f64::from(s.heading_deg).to_radians();
        let (len, rad) = (f64::from(s.length_m), f64::from(s.radius_m));
        let mut best = f32::MAX;
        for k in 0..=20 {
            let d = len * f64::from(k) / 20.0;
            let (px, py) = if rad.abs() > 1e-3 {
                let h = phi + d / rad;
                (
                    f64::from(s.position.x) - rad * (h.cos() - phi.cos()),
                    f64::from(s.position.y) + rad * (h.sin() - phi.sin()),
                )
            } else {
                (
                    f64::from(s.position.x) + d * phi.sin(),
                    f64::from(s.position.y) + d * phi.cos(),
                )
            };
            best = best.min(((px as f32 - x).powi(2) + (py as f32 - y).powi(2)).sqrt());
        }
        if best <= r {
            println!(
                "spline {:5.1} m away  z {:7.2} (terrain at start {:7.2})  len {:6.1} r {:7.1}  {}",
                best,
                s.position.z,
                th(s.position.x, s.position.y),
                s.length_m,
                s.radius_m,
                s.sli_file
            );
        }
    }
}
