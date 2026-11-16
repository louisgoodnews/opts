//! The terrain of every tile, carved under the ground surfaces of its own and its neighbours' splines.

use super::*;
use opts_bevy::terrain::{carve_terrain, spline_ground_triangles, TileTerrain};

/// The terrain each tile is drawn with: carved under the surfaces of its own and its eight
/// neighbours' splines, as the game does.
pub(super) fn carve_all(
    tiles: &Tiles,
    grid: &TileGrid,
    profile: &mut impl FnMut(&str) -> Option<SplineProfile>,
) -> HashMap<(i32, i32), TileTerrain> {
    let mut carved: HashMap<(i32, i32), TileTerrain> = HashMap::new();
    let (mut lowered, mut guarded) = (0, 0);
    for (key, (_, terrain)) in tiles {
        let Some(t) = terrain else { continue };
        let mut surfaces = Vec::new();
        for dx in -1..=1 {
            for dy in -1..=1 {
                let other = (key.0 + dx, key.1 + dy);
                let Some((n, _)) = tiles.get(&other) else {
                    continue;
                };
                let o = grid.neighbour_offset(*key, other);
                for s in &n.splines {
                    if let Some(p) = profile(&s.sli_file) {
                        surfaces.extend(spline_ground_triangles(&p, s, (o.x, o.y)));
                    }
                }
            }
        }
        let (c, stats) = carve_terrain(
            &TileTerrain::new(t.clone(), grid.cell(key.1) as f32),
            &surfaces,
        );
        lowered += stats.lowered;
        guarded += stats.guarded;
        carved.insert(*key, c);
    }
    println!("grid {grid:?}; vertices lowered {lowered}, left by the guard {guarded}");
    carved
}
