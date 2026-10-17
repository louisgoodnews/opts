//! Map scenario manifest (`global.cfg`), tile (`tile_*.map`), terrain, and water parsers, plus
//! writers for `global.cfg` and tile `[object]` records.

pub mod error;
pub mod global;
pub mod grid;
pub mod lists;
pub mod terrain;
pub mod tile;
pub mod water;
pub mod write;

pub use error::MapParseError;
pub use global::{MapEntryPoint, MapGroundTex, MapManifest, MapSeason, MapTileEntry};
pub use grid::{mercator_row_width, GridFit, TileGrid, DEFAULT_TILE_SIZE_M};
pub use lists::{
    parse_unsched_trafficdens, parse_unsched_vehgroups, AiEntry, AiGroup, AiLists, DensityProfile,
    GroupDensity, UnschedGroup,
};
pub use terrain::MapTerrain;
pub use tile::{
    spline_height_offset, MapAttachedObject, MapObjectInstance, MapSplineAttachment,
    MapSplineInstance, MapTile, MapTrafficRule, RuleKind, RuleOwner,
};
pub use water::MapWater;
pub use write::MapWriteError;
