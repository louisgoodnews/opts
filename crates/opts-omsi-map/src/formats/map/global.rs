//! Parser and data structures for OMSI map scenario manifests (`global.cfg`).
//!
//! `global.cfg` is the top-level manifest defining a map in OMSI. It declares
//! the map name, tile list, ground textures, seasons, traffic density curves,
//! money system bindings, and player spawn entry points.
//!
//! See specification in `docs/src/formats/map/global-cfg.md`.

use glam::Vec3;

use super::error::MapParseError;
use crate::formats::text::decode_bytes;

mod parse;

/// Ground texture surface configuration (`[groundtex]`).
#[derive(Debug, Clone, PartialEq)]
pub struct MapGroundTex {
    /// Base ground texture bitmap filename.
    pub ground_texture: String,
    /// Detail texture bitmap filename.
    pub detail_texture: String,
    /// Numeric surface/scale parameters.
    pub params: [f32; 3],
}

/// Calendar season date mapping (`[addseason]`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapSeason {
    /// Season ID (1 = Spring, 2 = Autumn, 3 = Winter, 4 = Winter Snow).
    pub season_id: u32,
    /// Starting day of year (0..370).
    pub start_day: u32,
    /// Ending day of year (0..370).
    pub end_day: u32,
}

/// Player/AI vehicle entry spawn point (`[entrypoints]`).
#[derive(Debug, Clone, PartialEq)]
pub struct MapEntryPoint {
    /// Entrypoint numeric identifier.
    pub id: u32,
    /// Object ID code on the tile.
    pub id_code: u32,
    /// Tile-local position in meters.
    pub position: Vec3,
    /// Normalized orientation direction vector.
    pub direction: Vec3,
    /// Friendly display name.
    pub name: String,
}

/// Reference to a map tile file (`[map]`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapTileEntry {
    /// Tile grid coordinate X (easting cell).
    pub x: i32,
    /// Tile grid coordinate Y (northing cell).
    pub y: i32,
    /// Tile filename (`tile_<x>_<y>.map`).
    pub file_name: String,
}

/// Complete representation of an OMSI map scenario manifest (`global.cfg`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MapManifest {
    /// Internal map identifier / folder name.
    pub name: String,
    /// Human-readable friendly name.
    pub friendly_name: Option<String>,
    /// Multi-line description text lines.
    pub description: Vec<String>,
    /// Map version number.
    pub version: Option<u32>,
    /// Next available object ID code.
    pub next_id_code: Option<u32>,
    /// Path to money system configuration (`Money/<pack>/<pack>.cfg`).
    pub money_system: Option<String>,
    /// Path to ticket pack configuration.
    pub ticket_pack: Option<String>,
    /// Default vehicle depot name.
    pub standard_depot: Option<String>,
    /// Valid calendar year range: `(first_year, last_year)`.
    pub years: Option<(i32, i32)>,
    /// Calendar year offset.
    pub real_year_offset: Option<i32>,
    /// Average vehicle maintenance/repair time in minutes.
    pub repair_time_min: Option<f32>,
    /// Whether world GPS coordinates are enabled.
    pub world_coordinates: bool,
    /// Whether realistic railway signaling rules apply.
    pub real_rail: bool,
    /// Whether dynamic helper arrows are active.
    pub dyn_helper_active: bool,
    /// Grid tiles comprising this map.
    pub tiles: Vec<MapTileEntry>,
    /// Vehicle entry and spawn points.
    pub entry_points: Vec<MapEntryPoint>,
    /// Ground texture palette mappings.
    pub ground_textures: Vec<MapGroundTex>,
    /// Calendar season definitions.
    pub seasons: Vec<MapSeason>,
    /// Time-of-day traffic density curve for road vehicles: `(hour_0_24, factor)`.
    pub traffic_density_road: Vec<(f32, f32)>,
    /// Time-of-day traffic density curve for pedestrians: `(hour_0_24, factor)`.
    pub traffic_density_passenger: Vec<(f32, f32)>,
}

impl MapManifest {
    /// Parses a map scenario manifest from raw file bytes.
    ///
    /// Automatically handles UTF-16LE/BE BOM, UTF-8 BOM, UTF-8, and CP1252 encodings.
    ///
    /// # Errors
    /// Returns [`MapParseError`] if decoding fails or numeric fields are malformed.
    pub fn parse(bytes: &[u8]) -> Result<Self, MapParseError> {
        let text = decode_bytes(bytes);
        Self::from_text(&text)
    }

    /// Parses a map scenario manifest from decoded text.
    ///
    /// # Errors
    /// Returns [`MapParseError`] if required parameters are missing or malformed.
    pub fn from_text(text: &str) -> Result<Self, MapParseError> {
        parse::parse_manifest(text)
    }
}

#[cfg(test)]
mod tests;
