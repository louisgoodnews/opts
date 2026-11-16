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

mod coords;
mod measure;

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

#[cfg(test)]
mod tests;
