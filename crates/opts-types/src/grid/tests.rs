//! Unit tests for `super` (`grid.rs`).

use super::*;

mod mercator;

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
