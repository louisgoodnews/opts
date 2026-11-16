//! Parser and data structures for OMSI map tile files (`tile_<x>_<y>.map`).
//!
//! A map tile is a 300 m × 300 m world grid square containing placed scenery
//! objects (`[object]`), splines (`[spline]`, `[spline_h]`), traffic rules (`[rule]`),
//! and spline attachments.
//!
//! See specification in `docs/src/formats/map/tiles.md`.

use glam::Vec3;

use super::error::MapParseError;
use crate::formats::text::{decode_bytes, parse_float, parse_keyword, OmsiBlock};
/// Placed scenery object instance on a tile (`[object]`).
#[derive(Debug, Clone, PartialEq)]
pub struct MapObjectInstance {
    /// Object placement flag (typically 0).
    pub flag: i32,
    /// Relative path to `.sco` file.
    pub sco_file: String,
    /// Unique object identifier code.
    pub id_code: u64,
    /// 3D placement coordinates in meters (X = easting, Y = northing, Z = elevation).
    pub position: Vec3,
    /// Heading (yaw) in degrees around vertical axis.
    pub heading_deg: f32,
    /// Pitch angle in degrees (None in legacy v10 tiles).
    pub pitch_deg: Option<f32>,
    /// Roll angle in degrees (None in legacy v10 tiles).
    pub roll_deg: Option<f32>,
    /// Per-instance string variable parameters (destination texts, bus stop names, etc.).
    pub string_vars: Vec<String>,
}

mod rules;
pub use rules::{MapTrafficRule, RuleKind, RuleOwner};

/// Complete representation of an OMSI tile file (`tile_<x>_<y>.map`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MapTile {
    /// Tile file format version (e.g. 14, 10).
    pub version: Option<u32>,
    /// Whether this tile contains a companion `.terrain` height grid.
    pub has_terrain: bool,
    /// Whether this tile contains a companion `.water` water level.
    pub has_water: bool,
    /// Whether variable terrain is enabled.
    pub has_variable_terrain: bool,
    /// Whether variable terrain lightmap is enabled.
    pub has_variable_terrain_lightmap: bool,
    /// ID codes of the splines marked `[spline_terrain_align]` / `[spline_terrain_align_2]` (value
    /// `1`, the block right after the spline): OMSI shapes the terrain to their `[heightprofile]`,
    /// however deep (a road down into a trough or a tunnel mouth).
    pub terrain_aligned: Vec<u64>,
    /// Placed scenery objects on this tile.
    pub objects: Vec<MapObjectInstance>,
    /// Placed spline segments on this tile.
    pub splines: Vec<MapSplineInstance>,
    /// Traffic rules declared on splines.
    pub rules: Vec<MapTrafficRule>,
    /// Attachments bound to splines.
    pub spline_attachments: Vec<MapSplineAttachment>,
    /// Objects placed on attachment points of other objects (`[attachObj]`).
    pub attached_objects: Vec<MapAttachedObject>,
    /// Records that could not be used and were left out, with their line numbers (truncated or
    /// damaged records, attachments without a carrier spline). The game logs them at WARN.
    pub warnings: Vec<String>,
    /// `[varparent] <id>` links: `(index into objects, parent id code)`. A signal head names its
    /// junction this way; its single string variable is the junction's light program index
    /// (`docs/src/architecture/scenery-runtime.md`).
    pub var_parents: Vec<(usize, u64)>,
}

impl MapTile {
    /// Parses a map tile from raw file bytes.
    ///
    /// Automatically handles UTF-16LE/BE BOM, UTF-8 BOM, UTF-8, and CP1252 encodings.
    ///
    /// # Errors
    /// Returns [`MapParseError`] if decoding fails or numeric fields are malformed.
    pub fn parse(bytes: &[u8]) -> Result<Self, MapParseError> {
        let text = decode_bytes(bytes);
        Self::from_text(&text)
    }
}

mod attached;
mod spline;

pub use attached::{MapAttachedObject, MapSplineAttachment, RepeaterLink};
pub use spline::{spline_height_offset, MapSplineInstance};
use spline::spline_end_pose;

mod attachment;
mod parse;
mod scan;

#[cfg(test)]
mod tests;
