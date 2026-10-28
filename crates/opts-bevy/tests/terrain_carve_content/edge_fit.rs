//! Content test for terrain alignment along the outer edges of road surfaces on Berlin-Spandau.

use super::*;

/// How far the terrain beside the outer edges of the road surfaces lies from the edge height
/// after the carve and the alignment, on the tile of Berlin-Spandau's Westerwaldstraße start
/// point (the project lead's report 2026-10-24: the terrain must fit the outer edges).
#[test]
fn spandau_terrain_beside_the_surface_edges_fits_their_height() {
    let Some(mut map) = Map::open("Berlin-Spandau") else {
        eprintln!("OMSI_PATH not set or Berlin-Spandau not installed; skipping");
        return;
    };
    let key = (2402, 11279);
    let raw = map.terrain(key).expect("terrain");
    let mut surfaces: Vec<GroundTriangle> = Vec::new();
    for dy in -1..=1 {
        for dx in -1..=1 {
            let other = (key.0 + dx, key.1 + dy);
            let Some(tile) = map.tile(other) else {
                continue;
            };
            let o = map.grid.neighbour_offset(key, other);
            for s in &tile.splines {
                if let Some(p) = map.sli(&s.sli_file) {
                    surfaces.extend(spline_ground_triangles(&p, s, (o.x, o.y)));
                }
            }
        }
    }
    let (carved, _) = carve_terrain(&raw, &surfaces);
    // Outer edges with the side that is outside (away from the triangle's third corner).
    type P = (i64, i64, i64);
    let k = |v: bevy::math::Vec3| -> P {
        (
            (v.x * 100.0).round() as i64,
            (v.y * 100.0).round() as i64,
            (v.z * 100.0).round() as i64,
        )
    };
    let mut count: HashMap<(P, P), (u8, bevy::math::Vec3, bevy::math::Vec3, bevy::math::Vec3)> =
        HashMap::new();
    for t in &surfaces {
        for e in 0..3 {
            let (a, b, c) = (t[e], t[(e + 1) % 3], t[(e + 2) % 3]);
            let (ka, kb) = (k(a), k(b));
            if ka == kb {
                continue;
            }
            let key2 = if ka < kb { (ka, kb) } else { (kb, ka) };
            let entry = count.entry(key2).or_insert((0, a, b, c));
            entry.0 = entry.0.saturating_add(1);
        }
    }
    let mut gaps: Vec<Vec<f32>> = vec![Vec::new(); 4];
    let dists = [0.3_f32, 1.0, 2.0, 3.0];
    for (n, a, b, c) in count.values() {
        if *n != 1 {
            continue;
        }
        let ab = (*b - *a).truncate();
        let len = ab.length();
        if len < 0.5 {
            continue;
        }
        let dir = ab / len;
        let mut normal = Vec2::new(-dir.y, dir.x);
        if normal.dot((*c - *a).truncate()) > 0.0 {
            normal = -normal;
        }
        let mut s = 0.5;
        while s < len {
            let p = a.truncate() + dir * s;
            let h = a.z + (b.z - a.z) * (s / len);
            for (i, d) in dists.iter().enumerate() {
                let q = p + normal * *d;
                if (1.0..299.0).contains(&q.x) && (1.0..299.0).contains(&q.y) {
                    gaps[i].push(carved.height(q.x, q.y) - h);
                }
            }
            s += 1.0;
        }
    }
    for (i, d) in dists.iter().enumerate() {
        // Gaps of more than 0.75 m are content (embankments, cuttings, bridges): the fit leaves
        // them, so they are not counted here.
        let g = &mut gaps[i];
        g.retain(|x| x.abs() <= 0.75);
        if g.is_empty() {
            continue;
        }
        g.sort_by(f32::total_cmp);
        let pct = |p: f32| g[((g.len() - 1) as f32 * p) as usize];
        let over =
            |t: f32| g.iter().filter(|x| x.abs() > t).count() as f32 / g.len() as f32 * 100.0;
        if i == 0 {
            assert!(
                pct(0.5).abs() < 0.05 && over(0.20) < 12.0,
                "the terrain 0.3 m outside the edges is off by {:+.3} m (median), {:.1} % by more than 20 cm",
                pct(0.5),
                over(0.20)
            );
        }
        eprintln!(
            "{d:.1} m outside: n={} p05 {:+.3} p50 {:+.3} p95 {:+.3}; |gap|>5cm {:.1} %, >20cm {:.1} %, >50cm {:.1} %",
            g.len(),
            pct(0.05),
            pct(0.5),
            pct(0.95),
            over(0.05),
            over(0.20),
            over(0.50)
        );
    }
}
