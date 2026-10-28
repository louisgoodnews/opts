//! Unit tests for `super` (`grid.rs`).

use super::*;

#[derive(Clone)]
struct TestSpline {
    id: u64,
    next: u64,
    x: f32,
    y: f32,
    heading: f32,
}

impl BorderSpline for TestSpline {
    fn id_code(&self) -> u64 {
        self.id
    }
    fn next_id_code(&self) -> u64 {
        self.next
    }
    fn start(&self) -> (f64, f64) {
        (f64::from(self.x), f64::from(self.y))
    }
    fn heading_deg(&self) -> f64 {
        f64::from(self.heading)
    }
    fn length_m(&self) -> f64 {
        100.0
    }
    fn radius_m(&self) -> f64 {
        0.0
    }
}

fn spline(id: u64, next: u64, x: f32, y: f32, heading: f32) -> TestSpline {
    TestSpline {
        id,
        next,
        x,
        y,
        heading,
    }
}

fn chain(pitch: f32, n: u64) -> Vec<((i32, i32), Vec<TestSpline>)> {
    // North-running chains: spline i ends at y = 350 of tile (0,0) and continues in (0,1).
    let a: Vec<_> = (0..n)
        .map(|i| spline(i + 1, i + 1000, 10.0 * i as f32, 250.0, 0.0))
        .collect();
    let b: Vec<_> = (0..n)
        .map(|i| spline(i + 1000, 0, 10.0 * i as f32, 350.0 - pitch, 0.0))
        .collect();
    vec![((0, 0), a), ((0, 1), b)]
}

#[test]
fn larger_tiles_are_measured() {
    let (grid, fit) = TileGrid::measure(&chain(372.0, 8), false);
    let TileGrid::Uniform { pitch } = grid else {
        panic!("{grid:?}")
    };
    assert!((pitch - 372.0).abs() < 1e-3);
    assert_eq!(fit.pairs, 8);
}

#[test]
fn ordinary_maps_and_sparse_data_keep_the_default() {
    assert_eq!(
        TileGrid::measure(&chain(300.0, 8), false).0,
        TileGrid::default()
    );
    assert_eq!(
        TileGrid::measure(&chain(372.0, 3), false).0,
        TileGrid::default()
    );
    // The flag alone does not select Mercator: the chains must agree with it.
    assert_eq!(
        TileGrid::measure(&chain(300.0, 8), true).0,
        TileGrid::default()
    );
}

/// 300 m maps keep today's `f32` origins bit for bit: `key as f32 · 300 − anchor as f32 · 300`.
#[test]
fn uniform_origins_are_bit_identical_to_the_f32_formula() {
    let grid = TileGrid::default();
    for anchor in [(0, 0), (-7, 3), (1000, -1000), (-1000, 1000), (17, 999)] {
        for k in -1000..=1000 {
            for key in [(k, anchor.1), (anchor.0, k), (k, -k)] {
                let old = (
                    key.0 as f32 * 300.0 - anchor.0 as f32 * 300.0,
                    key.1 as f32 * 300.0 - anchor.1 as f32 * 300.0,
                );
                let new = grid.offset(key, anchor);
                assert_eq!(new.x.to_bits(), old.0.to_bits(), "{key:?} {anchor:?}");
                assert_eq!(new.y.to_bits(), old.1.to_bits(), "{key:?} {anchor:?}");
            }
        }
    }
    assert_eq!(grid.cell(5) as f32, 5.0);
}

#[test]
fn mercator_rows_have_the_measured_widths() {
    // Berlin-Spandau rows 11271 and 11288 (tiles.md: 372.2 m to 371.7 m).
    assert!((mercator_row_width(11271) - 372.2).abs() < 0.05);
    assert!((mercator_row_width(11288) - 371.7).abs() < 0.05);
    assert!(mercator_row_width(11271) > mercator_row_width(11288));
}

/// Chains laid out on a synthetic 3 × 3 Mercator grid (rows 11278-11280): the grid is chosen
/// and the chains join their continuations within 1 cm in the session frame: every east-west
/// seam, and the north-south seams of the anchor column. Columns beside the anchor share their
/// x origin only in their own frames (zero shear), so their north-south seams open by the width
/// difference of the two rows per column (≈ 2.8 cm, the fixed-anchor error of the design).
#[test]
fn a_synthetic_mercator_grid_is_selected_and_its_seams_close() {
    let m = TileGrid::Mercator;
    let keys: Vec<(i32, i32)> = (11278..=11280)
        .flat_map(|ty| (2401..=2403).map(move |tx| (tx, ty)))
        .collect();
    let mut tiles: HashMap<(i32, i32), Vec<TestSpline>> = HashMap::new();
    let mut id = 1;
    for &(tx, ty) in &keys {
        let w = m.step_ew(ty) as f32;
        // East-running chains into (tx + 1, ty).
        if keys.contains(&(tx + 1, ty)) {
            for k in 0..3 {
                let y = 50.0 + 100.0 * k as f32;
                tiles
                    .entry((tx, ty))
                    .or_default()
                    .push(spline(id, id + 1, w - 60.0, y, 90.0));
                tiles
                    .entry((tx + 1, ty))
                    .or_default()
                    .push(spline(id + 1, 0, 40.0, y, 90.0));
                id += 2;
            }
        }
        // North-running chains into (tx, ty + 1).
        if keys.contains(&(tx, ty + 1)) {
            let step = m.step_ns(ty) as f32;
            for k in 0..3 {
                let x = 50.0 + 100.0 * k as f32;
                tiles
                    .entry((tx, ty))
                    .or_default()
                    .push(spline(id, id + 1, x, w - 70.0, 0.0));
                tiles.entry((tx, ty + 1)).or_default().push(spline(
                    id + 1,
                    0,
                    x,
                    w - 70.0 + 100.0 - step,
                    0.0,
                ));
                id += 2;
            }
        }
    }
    let sample: Vec<_> = tiles.into_iter().collect();
    let (grid, fit) = TileGrid::measure(&sample, true);
    assert_eq!(grid, TileGrid::Mercator, "{fit:?}");
    // Without the flag the same data falls back to one measured pitch.
    assert!(matches!(
        TileGrid::measure(&sample, false).0,
        TileGrid::Uniform { .. }
    ));
    let anchor = (2402, 11279);
    let by_id: HashMap<u64, ((i32, i32), TestSpline)> = sample
        .iter()
        .flat_map(|(t, v)| v.iter().map(move |s| (s.id_code(), (*t, s.clone()))))
        .collect();
    let mut joins = 0;
    for (tile, s) in by_id.values() {
        let Some((next_tile, next)) = by_id.get(&s.next_id_code()) else {
            continue;
        };
        let (ex, ey) = end_of(s);
        let a = grid.origin(*tile, anchor) + DVec2::new(ex, ey);
        let b = grid.origin(*next_tile, anchor) + DVec2::new(next.start().0, next.start().1);
        let columns = f64::from((tile.0 - anchor.0).abs());
        let allowed = if tile.1 == next_tile.1 || columns == 0.0 {
            0.01
        } else {
            let rows = (tile.1.min(next_tile.1), tile.1.max(next_tile.1));
            0.01 + columns * (grid.step_ew(rows.0) - grid.step_ew(rows.1)).abs()
        };
        assert!(
            a.distance(b) < allowed,
            "{tile:?} -> {next_tile:?}: {a} {b}"
        );
        joins += 1;
    }
    assert_eq!(joins, 36);
}

#[test]
fn tile_at_inverts_origin() {
    for grid in [TileGrid::default(), TileGrid::Mercator] {
        let anchor = (2402, 11279);
        for key in [(2402, 11279), (2400, 11277), (2405, 11283), (2399, 11280)] {
            let local = DVec2::new(12.5, 200.25);
            let p = grid.origin(key, anchor) + local;
            let (k, l) = grid.tile_at(p, anchor);
            assert_eq!(k, key, "{grid:?}");
            assert!((l - local).length() < 1e-6);
            let abs = grid.local_to_absolute(p, anchor);
            assert!((grid.absolute_to_local(abs, anchor) - p).length() < 1e-6);
        }
    }
    // Uniform: absolute coordinates are key · pitch + local.
    let abs = TileGrid::default().absolute((3, -2), DVec2::new(1.0, 2.0));
    assert_eq!(abs, DVec2::new(901.0, -598.0));
}
