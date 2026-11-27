//! The ground index: the resident tiles' plates, terrain and road samples, by cell.

use super::*;

impl GroundIndex {
    /// Sets the session frame: the start tile and the grid its positions are measured with. The
    /// terrain service follows, so it is set before the first tile is inserted.
    pub fn set_frame(&mut self, start_tile: (i32, i32), grid: TileGrid) {
        self.start_tile = start_tile;
        self.grid = grid;
        self.terrain.set_frame(grid, start_tile);
    }
}

/// A surface plate's triangle with the tile it came from.
pub(super) type TilePlateTri = (
    (i32, i32),
    std::sync::Arc<super::super::plate_surf::SurfTri>,
);

/// A road sample with the tile it came from.
pub(super) type TileSample = ((i32, i32), RoadSample);

/// Road surfaces and terrain of the resident tiles, in the session frame (metres relative to the
/// start tile's corner, OMSI axes).
#[derive(Resource)]
pub struct GroundIndex {
    /// The drivable surfaces of `[surface]` objects (junction and street plates): boxes whose top
    /// is the ground, per tile.
    pub(super) plates: ColliderGrid<(i32, i32)>,
    /// The terrain of the resident tiles: the one height service the game samples (ISS-071).
    pub(super) terrain: GroundElevationQuery,
    pub(super) cells: HashMap<(i32, i32), Vec<TileSample>>,
    pub start_tile: (i32, i32),
    pub grid: TileGrid,
    /// The strips with a height map of each spline profile (by `.sli` name, M-0146).
    pub(super) surf: HashMap<String, std::sync::Arc<Vec<opts_road_surf::TexStrip>>>,
    /// The upward triangles of the surface plates by cell, with their tile (M-0146).
    pub(super) plate_tris: HashMap<(i32, i32), Vec<TilePlateTri>>,
}

impl Default for GroundIndex {
    fn default() -> Self {
        Self {
            plates: ColliderGrid::new(CELL_M),
            terrain: GroundElevationQuery::default(),
            cells: HashMap::new(),
            start_tile: (0, 0),
            grid: TileGrid::default(),
            surf: HashMap::new(),
            plate_tris: HashMap::new(),
        }
    }
}

pub(super) fn cell_of(x: f32, y: f32) -> (i32, i32) {
    ((x / CELL_M).floor() as i32, (y / CELL_M).floor() as i32)
}
