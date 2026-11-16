//! The steps along the strips' outer edges, where the strips rise above the ground.

use super::*;

/// Steps along the strips' outer edges, split where the grid's triangles change.
pub(super) fn add_steps(
    b: &mut Builder,
    (terrain, cut): (&MapTerrain, &TerrainCut),
    (cell, n, span): (f32, usize, f32),
) {
    let inside_tile = |p: Vec2| (0.0..=span).contains(&p.x) && (0.0..=span).contains(&p.y);
    for (a3, b3) in &cut.edges {
        let (a, bb) = (a3.truncate(), b3.truncate());
        let mut ts = vec![0.0, 1.0];
        let d = bb - a;
        for k in 0..=n {
            let g = k as f32 * cell;
            for (pa, pd) in [(a.x, d.x), (a.y, d.y)] {
                if pd.abs() > 1e-6 {
                    ts.push((g - pa) / pd);
                }
            }
        }
        for k in 0..=2 * n {
            // Diagonals of the split: x + y = (i + j + 1) cell.
            let g = k as f32 * cell;
            let s = d.x + d.y;
            if s.abs() > 1e-6 {
                ts.push((g - a.x - a.y) / s);
            }
        }
        ts.retain(|t| (0.0..=1.0).contains(t));
        ts.sort_by(f32::total_cmp);
        ts.dedup_by(|x, y| (*x - *y).abs() < 1e-5);
        for w in ts.windows(2) {
            let (p, q) = (a + d * w[0], a + d * w[1]);
            if !inside_tile((p + q) * 0.5) {
                continue;
            }
            let top = (
                cut.capped(a3.z + (b3.z - a3.z) * w[0], p.x, p.y),
                cut.capped(a3.z + (b3.z - a3.z) * w[1], q.x, q.y),
            );
            let ground = (
                terrain_height(terrain, cell, p.x, p.y),
                terrain_height(terrain, cell, q.x, q.y),
            );
            if (ground.0 - top.0).abs().max((ground.1 - top.1).abs()) < STEP_MIN_M {
                continue;
            }
            b.face(p, q, top, ground);
        }
    }
}
