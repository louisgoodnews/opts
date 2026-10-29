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

/// Placed spline instance on a tile (`[spline]`, `[spline_h]`).
#[derive(Debug, Clone, PartialEq)]
pub struct MapSplineInstance {
    /// Spline placement flag.
    pub flag: i32,
    /// Relative path to `.sli` profile file.
    pub sli_file: String,
    /// Unique spline identifier code.
    pub id_code: u64,
    /// Connected element identifier 1 (or 0 if unconnected).
    pub conn_id_1: u64,
    /// Connected element identifier 2 (or 0 if unconnected).
    pub conn_id_2: u64,
    /// 3D position in standard engine coordinates (X = easting, Y = northing, Z = elevation).
    ///
    /// In OMSI's raw `tile_*.map` format, Line 5 is X, Line 6 is Z (elevation), and Line 7 is Y.
    /// This coordinate ordering was experimentally verified via in-engine Mutation Kit 2.
    pub position: Vec3,
    /// Heading (yaw) in degrees around vertical axis.
    pub heading_deg: f32,
    /// Segment arc length in meters.
    pub length_m: f32,
    /// Curvature radius in meters (0 for straight segment).
    pub radius_m: f32,
    /// Longitudinal gradient at start as a slope ratio (`[spline]` parameter 11, percent / 100).
    pub gradient_start: f32,
    /// Longitudinal gradient at end as a slope ratio (parameter 12, percent / 100).
    pub gradient_end: f32,
    /// Whether this spline includes an explicit height profile (`[spline_h]`).
    pub has_height_profile: bool,
    /// Whether the profile is mirrored across the spline axis (trailing `mirror` token).
    pub mirrored: bool,
    /// Additional raw parameters preserved from the block.
    pub raw_params: Vec<String>,
}

impl opts_types::BorderSpline for MapSplineInstance {
    fn id_code(&self) -> u64 {
        self.id_code
    }
    fn next_id_code(&self) -> u64 {
        self.conn_id_2
    }
    fn start(&self) -> (f64, f64) {
        (f64::from(self.position.x), f64::from(self.position.y))
    }
    fn heading_deg(&self) -> f64 {
        f64::from(self.heading_deg)
    }
    fn length_m(&self) -> f64 {
        f64::from(self.length_m)
    }
    fn radius_m(&self) -> f64 {
        f64::from(self.radius_m)
    }
}

/// Height above the start of a spline's height profile after `s` of `length` metres.
///
/// With an explicit total height change `rise` (`[spline_h]` parameter 13) the profile is a cubic
/// Hermite ramp between the end gradients; otherwise the gradient changes linearly from start to
/// end (`rise = length · (g0 + g1) / 2`). The road mesh, footpaths and AI lanes share this rule
/// (`docs/src/formats/map/tiles.md`).
#[must_use]
pub fn spline_height_offset(
    gradient_start: f64,
    gradient_end: f64,
    rise: Option<f64>,
    s: f64,
    length: f64,
) -> f64 {
    let length = length.max(1e-6);
    let s = s.max(0.0).min(length);
    match rise {
        Some(h) => {
            let t = s / length;
            let (m0, m1) = (gradient_start * length, gradient_end * length);
            let (t2, t3) = (t * t, t * t * t);
            (t3 - 2.0 * t2 + t) * m0 + (-2.0 * t3 + 3.0 * t2) * h + (t3 - t2) * m1
        }
        None => gradient_start * s + (gradient_end - gradient_start) * s * s / (2.0 * length),
    }
}

impl MapSplineInstance {
    /// The explicit total height change of a `[spline_h]` record (parameter 13), when it has one
    /// that is not zero.
    #[must_use]
    pub fn explicit_rise(&self) -> Option<f32> {
        self.raw_params
            .get(2)
            .and_then(|v| v.trim().parse::<f32>().ok())
            .filter(|h| self.has_height_profile && h.abs() > 1e-6)
    }

    /// Height of the centre line above the spline's start after `s` metres (see
    /// [`spline_height_offset`]).
    #[must_use]
    pub fn height_offset(&self, s: f32) -> f32 {
        spline_height_offset(
            f64::from(self.gradient_start),
            f64::from(self.gradient_end),
            self.explicit_rise().map(f64::from),
            f64::from(s),
            f64::from(self.length_m.max(0.01)),
        ) as f32
    }
}

mod rules;
pub use rules::{MapTrafficRule, RuleKind, RuleOwner};

/// Object attached to a spline (`[splineAttachement]`) or continued from another tile
/// (`[splineAttachement_repeater]`): street furniture, fences, shrubs, parking bays.
///
/// Layout (version 14, counted over the installed corpus; `docs/src/formats/map/tiles.md`): flag,
/// [repeater only: master tile index, first instance index], `.sco`, ID, spline index, lateral
/// offset, height, distance, heading, `field8` (always `0`), `field9`, spacing, range, `field12`,
/// string count, strings. Version 10 has no `field8`, `field9` and `field12`.
///
/// An attachment stands for the instances `k` of [`Self::instances`] at the chainages
/// [`Self::chainage`]`(k) = distance + k · spacing`, measured from the start of the connected spline
/// chain the carrier belongs to (walk `conn_id_1` backwards, summing lengths). A master places the
/// instances that fall on its own tile; a repeater continues the same set on another tile from
/// instance [`RepeaterLink::first_instance`] on.
#[derive(Debug, Clone, PartialEq)]
pub struct MapSplineAttachment {
    /// Attached scenery object `.sco` path.
    pub sco_file: String,
    /// Attachment object ID code.
    pub id_code: u64,
    /// Index of the carrying spline among this tile's `[spline]` blocks.
    pub spline_index: usize,
    /// Lateral offset from the spline centreline in metres (positive = right of travel).
    pub lateral_m: f32,
    /// Height above the spline surface in metres.
    pub height_m: f32,
    /// Chainage of instance 0 in metres, from the start of the carrier's spline chain.
    pub distance_m: f32,
    /// Heading relative to the spline direction in degrees.
    pub heading_deg: f32,
    /// Unidentified field after the heading (always `0` in the corpus; `0` in version 10).
    pub field8: f32,
    /// Unidentified field (`0` or ±2–5 m in the corpus; `0` in version 10) ❓.
    pub field9: f32,
    /// Distance between consecutive instances in metres.
    pub spacing_m: f32,
    /// Length of the run of instances in metres (`0` = a single instance).
    pub range_m: f32,
    /// Unidentified field (`0` or `1` in the corpus; `0` in version 10) ❓.
    pub field12: f32,
    /// What continues a master of another tile (`[splineAttachement_repeater]` only).
    pub repeater: Option<RepeaterLink>,
    /// Per-instance string variables (blank values kept), as for `[object]`.
    pub string_vars: Vec<String>,
}

/// The two leading fields of `[splineAttachement_repeater]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepeaterLink {
    /// Index of the master's tile in the `global.cfg` `[map]` list (not needed for placement).
    pub master_tile: i64,
    /// Index of the first instance that falls on this repeater's spline.
    pub first_instance: u32,
}

impl MapSplineAttachment {
    /// The instance indices this record places: from `0` (master) or the repeater's first
    /// instance to `⌊range / spacing⌋`; a single instance when spacing or range is not positive.
    #[must_use]
    pub fn instances(&self) -> std::ops::RangeInclusive<u32> {
        let first = self.repeater.map_or(0, |r| r.first_instance);
        if self.spacing_m > 0.0 && self.range_m > 0.0 {
            // A small allowance: `range` is written as a multiple of `spacing` in floating point.
            let last = (f64::from(self.range_m) / f64::from(self.spacing_m) + 1e-4).floor();
            first..=(last.min(f64::from(u32::MAX)) as u32)
        } else {
            first..=first
        }
    }

    /// Whether the record is a run of instances (positive spacing and range).
    #[must_use]
    pub fn is_run(&self) -> bool {
        self.spacing_m > 0.0 && self.range_m > 0.0
    }

    /// Chainage of instance `k` from the start of the carrier's spline chain (metres).
    #[must_use]
    pub fn chainage(&self, k: u32) -> f64 {
        f64::from(self.distance_m) + f64::from(k) * f64::from(self.spacing_m.max(0.0))
    }
}

/// Object placed on an attachment point of another object (`[attachObj]`): line modules and
/// timetables on bus-stop signs, signal heads on their arms, plates on sign poles. 62,609 records
/// in the installed corpus; every parent lies on the same tile (an `[object]`, another
/// `[attachObj]` or a spline attachment). Layout (counted over the corpus): `flag`, `.sco`, ID, parent
/// ID, a field that is always `0`, the parent's attachment point index (0-based, file order of its
/// `[new_attachment]` blocks), heading, then in version 14 pitch and roll, then the string
/// variables as in `[object]` (count, strings).
#[derive(Debug, Clone, PartialEq)]
pub struct MapAttachedObject {
    /// Placement flag (`0`).
    pub flag: i32,
    /// Attached scenery object `.sco` path.
    pub sco_file: String,
    /// ID code of this object (other attached objects may name it as their parent).
    pub id_code: u64,
    /// ID code of the parent object.
    pub parent_id: u64,
    /// Index of the parent's attachment point.
    pub attachment: usize,
    /// Heading relative to the attachment point (degrees, clockwise like `[object]`).
    pub heading_deg: f32,
    /// Pitch (version 14 only).
    pub pitch_deg: Option<f32>,
    /// Roll (version 14 only).
    pub roll_deg: Option<f32>,
    /// Per-object string variables.
    pub string_vars: Vec<String>,
}

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

/// End pose `(x, y, z, heading_deg)` of a spline: straight or circular arc (`radius_m != 0`),
/// the end height rises by `length * (g_start + g_end) / 2`.
pub(super) fn spline_end_pose(s: &MapSplineInstance) -> (f32, f32, f32, f32) {
    let phi = f64::from(s.heading_deg).to_radians();
    let len = f64::from(s.length_m);
    let r = f64::from(s.radius_m);
    let (x0, y0) = (f64::from(s.position.x), f64::from(s.position.y));
    let (h1, x1, y1) = if r.abs() > 1e-3 {
        let h = phi + len / r;
        (
            h,
            x0 - r * (h.cos() - phi.cos()),
            y0 + r * (h.sin() - phi.sin()),
        )
    } else {
        (phi, x0 + len * phi.sin(), y0 + len * phi.cos())
    };
    let dz = len * f64::from(s.gradient_start + s.gradient_end) / 2.0;
    (
        x1 as f32,
        y1 as f32,
        (f64::from(s.position.z) + dz) as f32,
        h1.to_degrees().rem_euclid(360.0) as f32,
    )
}

mod attachment;
mod parse;
mod scan;

#[cfg(test)]
mod tests;
