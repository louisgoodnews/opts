//! Map scenario manifest (`global.cfg`), tile (`tile_*.map`), terrain, and water parsers.

pub mod error;
pub mod global;
pub mod terrain;
pub mod tile;
pub mod water;

pub use error::MapParseError;
pub use global::{MapEntryPoint, MapGroundTex, MapManifest, MapSeason, MapTileEntry};
pub use terrain::MapTerrain;
pub use tile::{
    MapObjectInstance, MapSplineAttachment, MapSplineInstance, MapTile, MapTrafficRule,
};
pub use water::MapWater;
