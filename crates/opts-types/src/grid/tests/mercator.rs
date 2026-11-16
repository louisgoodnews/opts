//! Mercator row widths and the synthetic Mercator grid selection.

use super::*;

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
