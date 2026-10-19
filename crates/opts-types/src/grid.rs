//! Tile grid of a map: where each tile lies relative to the others (`docs/src/architecture/streaming-coordinates.md`).
//!
//! Ordinary maps use 300 m tiles. Maps with `[worldcoordinates]` are laid out on Web-Mercator
//! zoom-16 tiles (OMSI `x = slippy_x − 32768`, `y = 32768 − slippy_y`): a tile's ground size is the
//! Mercator width of its row, so the east-west step depends on the row and the north-south step
//! between two rows is the mean of their widths (measured on the 306 border chains of
//! Berlin-Spandau: median residual 0.006 m / 0.003 m, zero shear). Nothing in `global.cfg` states
//! the size; a spline that continues in the neighbouring tile starts at `end − step` in that
//! tile's own coordinates, so the grid is checked against every such pair
//! (`docs/src/formats/map/tiles.md`).
//!
//! All positions are `f64`; the session frame is the offset from an anchor tile, converted to
//! `f32` only as that offset ([`TileGrid::offset`]).

use std::collections::HashMap;

use glam::{DVec2, Vec2};

/// A road piece that ends in one tile and continues in an edge neighbour: what
/// [`TileGrid::measure`] needs from a spline, so the grid does not depend on any file format.
/// The legacy reader (`opts-omsi`) implements it for its spline record.
pub trait BorderSpline {
    /// Identifier of this spline.
    fn id_code(&self) -> u64;
    /// Identifier of the spline this one continues into (0 when none).
    fn next_id_code(&self) -> u64;
    /// Start of the spline in its tile's frame (x east, y north), metres.
    fn start(&self) -> (f64, f64);
    /// Heading, degrees clockwise from north.
    fn heading_deg(&self) -> f64;
    /// Arc length (m).
    fn length_m(&self) -> f64;
    /// Curvature radius (m), 0 for a straight piece.
    fn radius_m(&self) -> f64;
}

/// Default tile size (m).
pub const DEFAULT_TILE_SIZE_M: f64 = 300.0;

/// Equatorial circumference of the Web-Mercator sphere (m).
const EARTH_CIRCUMFERENCE_M: f64 = 40_075_016.686;
/// Tiles per axis at zoom 16.
const ZOOM16_TILES: f64 = 65_536.0;
/// Largest median deviation of the measured border steps from the Mercator steps for which a
/// `[worldcoordinates]` map is laid out on the Mercator grid (m).
const MERCATOR_AGREEMENT_M: f64 = 0.05;
/// Fewest border chains a measurement needs.
const MIN_PAIRS: usize = 5;
/// Terrain cells per tile edge.
const TERRAIN_CELLS: f64 = 60.0;

/// Ground width of OMSI tile row `ty` on the Mercator grid: the width of slippy row
/// `32767 − ty` at its centre latitude (m).
#[must_use]
pub fn mercator_row_width(ty: i32) -> f64 {
    let slippy_y = 32_768.0 - f64::from(ty) - 0.5;
    let n = std::f64::consts::PI - 2.0 * std::f64::consts::PI * slippy_y / ZOOM16_TILES;
    let lat = n.sinh().atan();
    EARTH_CIRCUMFERENCE_M * lat.cos() / ZOOM16_TILES
}

/// Sum of the Mercator north-south steps from row 0 to row `ty` (m), memoised: absolute
/// positions on a Mercator map lie some 11,000 rows from row 0.
fn mercator_rows_from_zero(ty: i32) -> f64 {
    static CACHE: std::sync::OnceLock<std::sync::Mutex<HashMap<i32, f64>>> =
        std::sync::OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    if let Some(v) = cache.lock().ok().and_then(|c| c.get(&ty).copied()) {
        return v;
    }
    let step = |r: i32| 0.5 * (mercator_row_width(r) + mercator_row_width(r + 1));
    let v = if ty >= 0 {
        (0..ty).map(step).sum()
    } else {
        -(ty..0).map(step).sum::<f64>()
    };
    if let Ok(mut c) = cache.lock() {
        c.insert(ty, v);
    }
    v
}

/// How the tiles of a map are laid out.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TileGrid {
    /// Square tiles of one size: 300 m maps, or the measured median as a fallback.
    Uniform {
        /// Tile edge (m).
        pitch: f64,
    },
    /// Web-Mercator rows (`[worldcoordinates]` maps whose border chains agree with it).
    Mercator,
}

impl Default for TileGrid {
    fn default() -> Self {
        Self::Uniform {
            pitch: DEFAULT_TILE_SIZE_M,
        }
    }
}

/// What [`TileGrid::measure`] found.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GridFit {
    /// Border chains measured.
    pub pairs: usize,
    /// Median step over the pairs between 300 and 600 m (m), `None` without enough pairs.
    pub median_step: Option<f64>,
    /// Median |measured − Mercator step| (m), when the map has `[worldcoordinates]`.
    pub mercator_residual: Option<f64>,
}

/// A spline that ends in one tile and continues in an edge neighbour.
struct BorderPair {
    tile: (i32, i32),
    next: (i32, i32),
    /// Measured step between the two tiles' origins along the axis they differ in (m).
    step: f64,
}

fn end_of<S: BorderSpline>(s: &S) -> (f64, f64) {
    let h = s.heading_deg().to_radians();
    let (x, y) = s.start();
    let (len, r) = (s.length_m(), s.radius_m());
    if r.abs() > 1e-3 {
        let h2 = h + len / r;
        (x - r * (h2.cos() - h.cos()), y + r * (h2.sin() - h.sin()))
    } else {
        (x + len * h.sin(), y + len * h.cos())
    }
}

fn border_pairs<S: BorderSpline>(tiles: &[((i32, i32), Vec<S>)]) -> Vec<BorderPair> {
    let by_id: HashMap<u64, ((i32, i32), &S)> = tiles
        .iter()
        .flat_map(|(t, v)| v.iter().map(move |s| (s.id_code(), (*t, s))))
        .collect();
    let mut out = Vec::new();
    for (tile, s) in by_id.values() {
        let Some((next_tile, next)) = by_id.get(&s.next_id_code()) else {
            continue;
        };
        let (dx, dy) = (next_tile.0 - tile.0, next_tile.1 - tile.1);
        if dx.abs() + dy.abs() != 1 {
            continue;
        }
        let (ex, ey) = end_of(*s);
        let step = if dx != 0 {
            (ex - next.start().0) / f64::from(dx)
        } else {
            (ey - next.start().1) / f64::from(dy)
        };
        out.push(BorderPair {
            tile: *tile,
            next: *next_tile,
            step,
        });
    }
    out
}

fn median(mut v: Vec<f64>) -> Option<f64> {
    if v.is_empty() {
        return None;
    }
    v.sort_by(f64::total_cmp);
    Some(v[v.len() / 2])
}

impl TileGrid {
    /// Chooses the grid of a map from the splines of several of its tiles (`(tile, splines)`).
    ///
    /// `Mercator` when the map has `[worldcoordinates]` and the measured border steps agree with
    /// it within 0.05 m (median); otherwise `Uniform` with the measured median step (only steps
    /// between 300 and 600 m count), or 300 m when fewer than 5 chains cross a border or the
    /// median is within 0.5 m of 300.
    #[must_use]
    pub fn measure<S: BorderSpline>(
        tiles: &[((i32, i32), Vec<S>)],
        world_coordinates: bool,
    ) -> (Self, GridFit) {
        let pairs = border_pairs(tiles);
        let plausible: Vec<f64> = pairs
            .iter()
            .map(|p| p.step)
            .filter(|s| (300.0..600.0).contains(s))
            .collect();
        let median_step = if plausible.len() >= MIN_PAIRS {
            median(plausible)
        } else {
            None
        };
        let mercator_residual = if world_coordinates && pairs.len() >= MIN_PAIRS {
            median(
                pairs
                    .iter()
                    .map(|p| {
                        let expected = if p.tile.1 == p.next.1 {
                            Self::Mercator.step_ew(p.tile.1)
                        } else {
                            Self::Mercator.step_ns(p.tile.1.min(p.next.1))
                        };
                        (p.step - expected).abs()
                    })
                    .collect(),
            )
        } else {
            None
        };
        let fit = GridFit {
            pairs: pairs.len(),
            median_step,
            mercator_residual,
        };
        if mercator_residual.is_some_and(|r| r <= MERCATOR_AGREEMENT_M) {
            return (Self::Mercator, fit);
        }
        let pitch = match median_step {
            Some(m) if (m - DEFAULT_TILE_SIZE_M).abs() >= 0.5 => m,
            _ => DEFAULT_TILE_SIZE_M,
        };
        (Self::Uniform { pitch }, fit)
    }

    /// Edge length of the tiles of row `ty`, which is also the east-west step between two tiles of
    /// that row (m).
    #[must_use]
    pub fn step_ew(&self, ty: i32) -> f64 {
        match self {
            Self::Uniform { pitch } => *pitch,
            Self::Mercator => mercator_row_width(ty),
        }
    }

    /// North-south step from row `ty` to row `ty + 1` (m).
    #[must_use]
    pub fn step_ns(&self, ty: i32) -> f64 {
        match self {
            Self::Uniform { pitch } => *pitch,
            Self::Mercator => 0.5 * (mercator_row_width(ty) + mercator_row_width(ty + 1)),
        }
    }

    /// Edge length of a tile of row `ty` in its own frame (m); the same as [`Self::step_ew`].
    #[must_use]
    pub fn tile_size(&self, ty: i32) -> f64 {
        self.step_ew(ty)
    }

    /// Terrain cell of row `ty` (a tile has 60 × 60 cells).
    #[must_use]
    pub fn cell(&self, ty: i32) -> f64 {
        self.step_ew(ty) / TERRAIN_CELLS
    }

    /// Northing of row `ty`'s origins relative to row `ay` (m).
    fn row_offset(&self, ty: i32, ay: i32) -> f64 {
        match self {
            Self::Uniform { pitch } => f64::from(ty - ay) * pitch,
            Self::Mercator if (ty - ay).abs() <= 64 => {
                if ty >= ay {
                    (ay..ty).map(|r| self.step_ns(r)).sum()
                } else {
                    -(ty..ay).map(|r| self.step_ns(r)).sum::<f64>()
                }
            }
            Self::Mercator => mercator_rows_from_zero(ty) - mercator_rows_from_zero(ay),
        }
    }

    /// South-west corner of tile `key` relative to the corner of tile `anchor` (m).
    ///
    /// `x = (tx − ax) · step_ew(ty)`: tiles of one column share their x origin (zero shear);
    /// `y` sums the north-south steps from the anchor row.
    #[must_use]
    pub fn origin(&self, key: (i32, i32), anchor: (i32, i32)) -> DVec2 {
        DVec2::new(
            f64::from(key.0 - anchor.0) * self.step_ew(key.1),
            self.row_offset(key.1, anchor.1),
        )
    }

    /// [`Self::origin`] as the `f32` offset of the session frame.
    #[must_use]
    pub fn offset(&self, key: (i32, i32), anchor: (i32, i32)) -> Vec2 {
        self.origin(key, anchor).as_vec2()
    }

    /// Offset between two tiles: the corner of `other` in the frame of `key` (`f32`).
    #[must_use]
    pub fn neighbour_offset(&self, key: (i32, i32), other: (i32, i32)) -> Vec2 {
        self.origin(other, key).as_vec2()
    }

    /// The tile a point lies on and the point in that tile's frame; `local` is relative to the
    /// anchor tile's corner.
    #[must_use]
    pub fn tile_at(&self, local: DVec2, anchor: (i32, i32)) -> ((i32, i32), DVec2) {
        let ty = match self {
            Self::Uniform { pitch } => anchor.1 + (local.y / pitch).floor() as i32,
            Self::Mercator => {
                // Jump by the local row step until the row is found (a few rounds even from
                // row 0), then settle on the row whose band holds the point.
                let mut ty = anchor.1;
                for _ in 0..32 {
                    let jump = ((local.y - self.row_offset(ty, anchor.1)) / self.step_ns(ty))
                        .floor() as i32;
                    if jump == 0 {
                        break;
                    }
                    ty += jump;
                }
                while local.y < self.row_offset(ty, anchor.1) {
                    ty -= 1;
                }
                while local.y >= self.row_offset(ty + 1, anchor.1) {
                    ty += 1;
                }
                ty
            }
        };
        let w = self.step_ew(ty);
        let tx = anchor.0 + (local.x / w).floor() as i32;
        let key = (tx, ty);
        (key, local - self.origin(key, anchor))
    }

    /// [`Self::tile_at`] for an `f32` session-frame point, returning only the tile.
    #[must_use]
    pub fn key_at(&self, local: Vec2, anchor: (i32, i32)) -> (i32, i32) {
        self.tile_at(local.as_dvec2(), anchor).0
    }

    /// Absolute position of a point of tile `key` (its frame relative to tile (0, 0)), as the
    /// control protocol speaks it. On a uniform grid this is `key · pitch + local`.
    #[must_use]
    pub fn absolute(&self, key: (i32, i32), local: DVec2) -> DVec2 {
        self.origin(key, (0, 0)) + local
    }

    /// Session-frame position of an absolute position (see [`Self::absolute`]), for a session
    /// anchored at `anchor`.
    #[must_use]
    pub fn absolute_to_local(&self, abs: DVec2, anchor: (i32, i32)) -> DVec2 {
        let (key, local) = self.tile_at(abs, (0, 0));
        self.origin(key, anchor) + local
    }

    /// Absolute position of a session-frame position (see [`Self::absolute`]).
    #[must_use]
    pub fn local_to_absolute(&self, local: DVec2, anchor: (i32, i32)) -> DVec2 {
        let (key, in_tile) = self.tile_at(local, anchor);
        self.absolute(key, in_tile)
    }
}

#[cfg(test)]
mod tests {
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
}
