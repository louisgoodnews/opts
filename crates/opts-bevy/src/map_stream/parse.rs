//! The chunk job itself: read and parse one `tile_<x>_<y>.map` file.

use std::path::Path;

use opts_core::formats::map::MapTile;

/// Why a tile file did not give a tile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TileParseError {
    /// The file could not be read (missing, unreadable). Carries the path and the OS message.
    Read(String),
    /// The bytes could not be decoded or a numeric field is malformed. Carries the parser message.
    Parse(String),
}

/// The outcome of one parse job.
pub type TileParseResult = Result<MapTile, TileParseError>;

/// Parses the bytes of a tile file (UTF-16, UTF-8 and CP1252 are handled by [`MapTile::parse`]).
pub fn parse_tile_bytes(bytes: &[u8]) -> TileParseResult {
    MapTile::parse(bytes).map_err(|e| TileParseError::Parse(format!("{e:?}")))
}

/// Reads and parses the tile file at `path`. This is the body of one async chunk job.
pub fn parse_tile_file(path: &Path) -> TileParseResult {
    let bytes = std::fs::read(path)
        .map_err(|e| TileParseError::Read(format!("{}: {e}", path.display())))?;
    parse_tile_bytes(&bytes)
}
