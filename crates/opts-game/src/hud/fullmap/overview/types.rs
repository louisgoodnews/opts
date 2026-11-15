//! The types of the overview: a tile's roads, the listing, the stop sites and the reader state.

use super::*;

/// Pause between two tiles, so the thread stays out of the way of the streaming.
pub(super) const PAUSE_BETWEEN_TILES: std::time::Duration = std::time::Duration::from_millis(2);

/// The roads of one tile and the box that contains them.
#[derive(Debug, Clone)]
pub struct TileRoads {
    /// The lanes (world frame).
    pub roads: Vec<MapRoad>,
    /// South-west and north-east corner of the box around all lanes, metres.
    pub bounds: (DVec2, DVec2),
}

/// One tile of the listing: its key, its file name and its south-west corner (session frame).
pub type ListedTile = ((i32, i32), String, (f32, f32));

/// One answer of the reader thread.
/// A bus stop object: its ID code, position (east, north) and heading (radians, compass).
pub type StopSite = (u64, DVec2, f64);

pub(super) type Batch = ((i32, i32), Vec<MapRoad>, Vec<StopSite>);

/// The roads of the whole map as the reader found them.
#[derive(Resource, Default)]
pub struct OverviewRoads {
    pub(super) tiles: HashMap<(i32, i32), TileRoads>,
    pub(super) stops: HashMap<u64, (DVec2, f64)>,
    /// Tiles the reader will visit.
    pub total: usize,
    /// Tiles it has finished (including those without roads or without a readable file).
    pub done: usize,
    /// Whether a reader was started this session.
    pub(super) started: bool,
    pub(super) rx: Option<Mutex<Receiver<Batch>>>,
    /// Counted up whenever tiles arrive, so the map knows it has new roads to draw.
    pub generation: u64,
}
