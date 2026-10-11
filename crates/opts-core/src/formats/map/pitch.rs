//! Tile size of a map, measured from spline chains that cross tile borders.
//!
//! Ordinary maps use 300 m tiles. Maps with `[worldcoordinates]` use larger ones (Berlin-Spandau
//! 372 m, Wuppertal 639 382.8 m) and nothing in `global.cfg` says by how much: a spline that
//! continues in the neighbouring tile starts at `end − pitch · Δtile` in that tile's own
//! coordinates, so the pitch follows from every such pair (`docs/src/formats/map/tiles.md`).

use std::collections::HashMap;

use super::MapSplineInstance;

/// Default tile size (m).
pub const DEFAULT_TILE_SIZE_M: f32 = 300.0;

fn end_of(s: &MapSplineInstance) -> (f32, f32) {
    let h = s.heading_deg.to_radians();
    if s.radius_m.abs() > 1e-3 {
        let (r, h2) = (s.radius_m, h + s.length_m / s.radius_m);
        (
            s.position.x - r * (h2.cos() - h.cos()),
            s.position.y + r * (h2.sin() - h.sin()),
        )
    } else {
        (
            s.position.x + s.length_m * h.sin(),
            s.position.y + s.length_m * h.cos(),
        )
    }
}

/// Estimates the tile size from the splines of several tiles (`(tile, splines)`).
///
/// Returns [`DEFAULT_TILE_SIZE_M`] when fewer than 5 chains cross a border or the median is
/// within 0.5 m of it; otherwise the median over the crossing pairs (only values between 300 and
/// 600 m count).
#[must_use]
pub fn estimate_tile_size(tiles: &[((i32, i32), Vec<MapSplineInstance>)]) -> f32 {
    let by_id: HashMap<u64, ((i32, i32), &MapSplineInstance)> = tiles
        .iter()
        .flat_map(|(t, v)| v.iter().map(move |s| (s.id_code, (*t, s))))
        .collect();
    let mut est: Vec<f32> = Vec::new();
    for (tile, s) in by_id.values() {
        let Some((next_tile, next)) = by_id.get(&s.conn_id_2) else {
            continue;
        };
        let (dx, dy) = (next_tile.0 - tile.0, next_tile.1 - tile.1);
        if dx.abs() + dy.abs() != 1 {
            continue;
        }
        let (ex, ey) = end_of(s);
        let p = if dx != 0 {
            (ex - next.position.x) / dx as f32
        } else {
            (ey - next.position.y) / dy as f32
        };
        if (300.0..600.0).contains(&p) {
            est.push(p);
        }
    }
    if est.len() < 5 {
        return DEFAULT_TILE_SIZE_M;
    }
    est.sort_by(f32::total_cmp);
    let median = est[est.len() / 2];
    if (median - DEFAULT_TILE_SIZE_M).abs() < 0.5 {
        DEFAULT_TILE_SIZE_M
    } else {
        median
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    fn spline(id: u64, next: u64, x: f32, y: f32) -> MapSplineInstance {
        MapSplineInstance {
            flag: 0,
            sli_file: String::new(),
            id_code: id,
            conn_id_1: 0,
            conn_id_2: next,
            position: Vec3::new(x, y, 0.0),
            heading_deg: 0.0,
            length_m: 100.0,
            radius_m: 0.0,
            gradient_start: 0.0,
            gradient_end: 0.0,
            has_height_profile: false,
            mirrored: false,
            raw_params: Vec::new(),
        }
    }

    fn chain(pitch: f32, n: u64) -> Vec<((i32, i32), Vec<MapSplineInstance>)> {
        // North-running chains: spline i ends at y = 300.. of tile (0,0) and continues in (0,1).
        let a: Vec<_> = (0..n)
            .map(|i| spline(i + 1, i + 1000, 10.0 * i as f32, 250.0))
            .collect();
        let b: Vec<_> = (0..n)
            .map(|i| spline(i + 1000, 0, 10.0 * i as f32, 350.0 - pitch))
            .collect();
        vec![((0, 0), a), ((0, 1), b)]
    }

    #[test]
    fn larger_tiles_are_measured() {
        assert!((estimate_tile_size(&chain(372.0, 8)) - 372.0).abs() < 1e-3);
    }

    #[test]
    fn ordinary_maps_and_sparse_data_keep_the_default() {
        assert_eq!(estimate_tile_size(&chain(300.0, 8)), 300.0);
        assert_eq!(estimate_tile_size(&chain(372.0, 3)), 300.0);
    }
}
