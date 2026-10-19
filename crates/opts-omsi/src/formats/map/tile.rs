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

/// What a `[rule]` is attached to: the spline or object whose block precedes it in the tile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleOwner {
    /// A spline: `index` into [`MapTile::splines`].
    Spline {
        /// Position in [`MapTile::splines`].
        index: usize,
        /// The spline's `id_code`.
        id_code: u64,
    },
    /// A scenery object (a junction, say): `index` into [`MapTile::objects`].
    Object {
        /// Position in [`MapTile::objects`].
        index: usize,
        /// The object's `id_code`.
        id_code: u64,
    },
}

/// The known rule types of `[rule]` records (`docs/src/formats/map/tiles.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleKind {
    /// `trafficdensity`: AI traffic density on the path (0 = none).
    TrafficDensity,
    /// `speedlimit`: speed limit on the path.
    SpeedLimit,
    /// `priority`: priority of the path at a junction.
    Priority,
    /// `trucks`: vehicle-type restriction.
    Trucks,
    /// `overtaking_prohib`: overtaking prohibited.
    OvertakingProhibited,
    /// `bus`: vehicle-type restriction.
    Bus,
    /// `no_cars`: vehicle-type restriction.
    NoCars,
}

/// Traffic rule constraint on a path of a spline or object (`[rule]`).
#[derive(Debug, Clone, PartialEq)]
pub struct MapTrafficRule {
    /// The spline or object the rule follows in the file; `None` for a rule before any of them.
    pub owner: Option<RuleOwner>,
    /// Index of the path (lane) among the owner's `[path]` records.
    pub path_index: usize,
    /// Rule type identifier (e.g. `"trafficdensity"`, `"speedlimit"`, `"priority"`).
    pub rule_type: String,
    /// Numeric rule value (speed limit, priority mask, etc.).
    pub value: f32,
    /// Additional rule flag.
    pub flag: u32,
}

impl MapTrafficRule {
    /// The rule type, if it is one of the known ones (case-insensitive).
    #[must_use]
    pub fn kind(&self) -> Option<RuleKind> {
        Some(match self.rule_type.to_ascii_lowercase().as_str() {
            "trafficdensity" => RuleKind::TrafficDensity,
            "speedlimit" => RuleKind::SpeedLimit,
            "priority" => RuleKind::Priority,
            "trucks" => RuleKind::Trucks,
            "overtaking_prohib" => RuleKind::OvertakingProhibited,
            "bus" => RuleKind::Bus,
            "no_cars" => RuleKind::NoCars,
            _ => return None,
        })
    }
}

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

    /// Parses a map tile from decoded text.
    ///
    /// # Errors
    /// Returns [`MapParseError`] if required parameters are missing or malformed.
    pub fn from_text(text: &str) -> Result<Self, MapParseError> {
        let blocks = scan_tile_blocks(text);
        let mut tile = MapTile::default();
        // The spline or object a following `[rule]` belongs to.
        let mut rule_owner: Option<RuleOwner> = None;
        // The object a `[varparent]` right after it belongs to.
        let mut prev_object: Option<usize> = None;

        for raw in &blocks {
            let block = &raw.block;
            let this_object = prev_object.take();
            match block.keyword.as_str() {
                "varparent" => {
                    let parent = block.parameters.first().and_then(|s| s.parse().ok());
                    if let (Some(index), Some(parent)) = (this_object, parent) {
                        tile.var_parents.push((index, parent));
                    }
                }
                "version" => {
                    if let Some(val) = block.parameters.first() {
                        tile.version = Some(parse_u32(val, "version", block.line_number)?);
                    }
                }
                "terrain" => {
                    tile.has_terrain = true;
                }
                "water" => {
                    tile.has_water = true;
                }
                "variable_terrain" => {
                    tile.has_variable_terrain = true;
                }
                "variable_terrainlightmap" => {
                    tile.has_variable_terrain_lightmap = true;
                }
                "object" => {
                    // flag, .sco, ID, x, y, z, heading, [pitch, roll,] string count, strings.
                    let is_v10 = tile.version == Some(10);
                    let mut cur = raw.cursor();
                    let fixed = cur.fields(if is_v10 { 8 } else { 10 });
                    if fixed.len() < 7 {
                        tile.warnings.push(format!(
                            "line {}: [object] with {} fields skipped (damaged record): {}",
                            block.line_number,
                            fixed.len(),
                            fixed.get(1).map_or("", |s| s.trim_matches('\0'))
                        ));
                        continue;
                    }
                    let line = block.line_number;
                    let flag = parse_i32(fixed[0], "object flag", line)?;
                    let id = parse_u64(fixed[2], "object id", line)?;
                    let x = parse_f32(fixed[3], "object x", line)?;
                    let y = parse_f32(fixed[4], "object y", line)?;
                    let z = parse_f32(fixed[5], "object z", line)?;
                    let heading = parse_f32(fixed[6], "object heading", line)?;
                    let (pitch, roll, count) = if is_v10 {
                        (None, None, fixed.get(7))
                    } else {
                        (
                            fixed.get(7).and_then(|s| parse_float(s).ok()),
                            fixed.get(8).and_then(|s| parse_float(s).ok()),
                            fixed.get(9),
                        )
                    };
                    let string_vars = cur.strings(count.copied());
                    rule_owner = Some(RuleOwner::Object {
                        index: tile.objects.len(),
                        id_code: id,
                    });
                    prev_object = Some(tile.objects.len());
                    tile.objects.push(MapObjectInstance {
                        flag,
                        sco_file: fixed[1].to_string(),
                        id_code: id,
                        position: Vec3::new(x, y, z),
                        heading_deg: heading,
                        pitch_deg: pitch,
                        roll_deg: roll,
                        string_vars,
                    });
                }
                "spline" | "spline_h" => {
                    // Version 10 (older editor, 556 tiles of Neuendorf, Rheinhausen and Ahlheim 5)
                    // has one connection field instead of two and gives the placement only when
                    // that field is `-1`; `0` means "continues the previous spline" and carries
                    // zeros for the pose (P-1014, MI-0021). A virtual second connection `0` brings
                    // the record into the version 14 layout.
                    let is_v10 = tile.version == Some(10);
                    let chained_v10 =
                        is_v10 && block.parameters.get(3).is_some_and(|c| c.trim() != "-1");
                    let normalised: Vec<String> = if is_v10 && block.parameters.len() >= 4 {
                        let mut v = block.parameters.clone();
                        v.insert(4, "0".to_string());
                        v
                    } else {
                        block.parameters.clone()
                    };
                    let block = &OmsiBlock {
                        keyword: block.keyword.clone(),
                        line_number: block.line_number,
                        parameters: normalised,
                    };
                    if block.parameters.len() >= 11 {
                        let has_h = block.keyword == "spline_h";
                        let flag =
                            parse_i32(&block.parameters[0], "spline flag", block.line_number)?;
                        let sli = block.parameters[1].clone();
                        let id = parse_u64(&block.parameters[2], "spline id", block.line_number)?;
                        let c1 =
                            parse_conn(&block.parameters[3], "spline conn1", block.line_number)?;
                        let c2 =
                            parse_conn(&block.parameters[4], "spline conn2", block.line_number)?;
                        let px = parse_f32(&block.parameters[5], "spline x", block.line_number)?;
                        // CRITICAL: Line 6 is vertical elevation Z, Line 7 is horizontal Y
                        let pz = parse_f32(
                            &block.parameters[6],
                            "spline elevation z",
                            block.line_number,
                        )?;
                        let py = parse_f32(&block.parameters[7], "spline y", block.line_number)?;
                        let heading =
                            parse_f32(&block.parameters[8], "spline heading", block.line_number)?;
                        let length =
                            parse_f32(&block.parameters[9], "spline length", block.line_number)?;
                        let radius =
                            parse_f32(&block.parameters[10], "spline radius", block.line_number)?;

                        // Parameters 11 and 12 are the start and end gradient in percent
                        // (verified on 8,500 connected Bremen-Nord pairs: the next segment starts
                        // exactly `length * (p11 + p12) / 200` m higher for 100 % of `[spline]`
                        // records); stored here as slope ratios.
                        let grad_start = block
                            .parameters
                            .get(11)
                            .and_then(|s| parse_float(s).ok())
                            .unwrap_or(0.0)
                            / 100.0;
                        let grad_end = block
                            .parameters
                            .get(12)
                            .and_then(|s| parse_float(s).ok())
                            .unwrap_or(0.0)
                            / 100.0;

                        let (mut px, mut py, mut pz, mut heading) = (px, py, pz, heading);
                        if chained_v10 {
                            let Some(prev) = tile.splines.last() else {
                                // Nothing to continue: the pose is unknown.
                                tile.warnings.push(format!(
                                    "line {}: version 10 [spline] {} continues no spline: skipped",
                                    block.line_number, sli
                                ));
                                continue;
                            };
                            let end = spline_end_pose(prev);
                            (px, py, pz, heading) = (end.0, end.1, end.2, end.3);
                        }
                        rule_owner = Some(RuleOwner::Spline {
                            index: tile.splines.len(),
                            id_code: id,
                        });
                        tile.splines.push(MapSplineInstance {
                            flag,
                            sli_file: sli,
                            id_code: id,
                            conn_id_1: c1,
                            conn_id_2: c2,
                            position: Vec3::new(px, py, pz),
                            heading_deg: heading,
                            length_m: length,
                            radius_m: radius,
                            gradient_start: grad_start,
                            gradient_end: grad_end,
                            has_height_profile: has_h,
                            mirrored: block
                                .parameters
                                .iter()
                                .skip(13)
                                .any(|p| p.trim().eq_ignore_ascii_case("mirror")),
                            raw_params: block.parameters[11..].to_vec(),
                        });
                    } else {
                        tile.warnings.push(format!(
                            "line {}: [{}] with {} fields skipped (damaged record)",
                            block.line_number,
                            block.keyword,
                            block.parameters.len()
                        ));
                    }
                }
                "rule" => {
                    // Version 10 rules have no flag line (index, type, value only).
                    let v10_rule = tile.version == Some(10);
                    if block.parameters.len() >= 4 || (v10_rule && block.parameters.len() >= 3) {
                        let idx = parse_usize(
                            &block.parameters[0],
                            "rule path_index",
                            block.line_number,
                        )?;
                        let rtype = block.parameters[1].clone();
                        let val = parse_f32(&block.parameters[2], "rule value", block.line_number)?;
                        let flag = if v10_rule {
                            block
                                .parameters
                                .get(3)
                                .and_then(|p| p.trim().parse::<u32>().ok())
                                .unwrap_or(0)
                        } else {
                            parse_u32(&block.parameters[3], "rule flag", block.line_number)?
                        };
                        tile.rules.push(MapTrafficRule {
                            owner: rule_owner,
                            path_index: idx,
                            rule_type: rtype,
                            value: val,
                            flag,
                        });
                    }
                }
                "attachobj" => {
                    let v14 = tile.version != Some(10);
                    let strings_at = if v14 { 9 } else { 7 };
                    let mut cur = raw.cursor();
                    let p = cur.fields(strings_at + 1);
                    if p.len() < strings_at {
                        tile.warnings.push(format!(
                            "line {}: [attachObj] with {} fields skipped (damaged record): {}",
                            block.line_number,
                            p.len(),
                            p.get(1).copied().unwrap_or("")
                        ));
                        continue;
                    }
                    let line = block.line_number;
                    tile.attached_objects.push(MapAttachedObject {
                        flag: parse_i32(p[0], "attachobj flag", line)?,
                        sco_file: p[1].to_string(),
                        id_code: parse_u64(p[2], "attachobj id", line)?,
                        parent_id: parse_u64(p[3], "attachobj parent", line)?,
                        attachment: parse_u32(p[5], "attachobj point", line)? as usize,
                        heading_deg: parse_f32(p[6], "attachobj heading", line)?,
                        pitch_deg: v14.then(|| parse_float(p[7]).ok()).flatten(),
                        roll_deg: v14.then(|| parse_float(p[8]).ok()).flatten(),
                        string_vars: cur.strings(p.get(strings_at).copied()),
                    });
                }
                "splineattachement" | "splineattachement_repeater" => {
                    let repeater = block.keyword == "splineattachement_repeater";
                    let o = if repeater { 2 } else { 0 };
                    let v10 = tile.version == Some(10);
                    // Version 14: … heading, field8, field9, spacing, range, field12, count;
                    // version 10: … heading, spacing, range, count.
                    let count_at = o + if v10 { 10 } else { 13 };
                    let mut cur = raw.cursor();
                    let p = cur.fields(count_at + 1);
                    if p.len() < 8 + o {
                        tile.warnings.push(format!(
                            "line {}: [{}] with {} fields skipped (damaged record)",
                            block.line_number,
                            block.keyword,
                            p.len()
                        ));
                        continue;
                    }
                    // `-1` as carrier: the attachment belongs to no spline (an editor leftover, e.g.
                    // Berlin-Spandau tile 2394/11275). It has no place to be put.
                    if p[3 + o].trim() == "-1" {
                        tile.warnings.push(format!(
                            "line {}: attachment {} names no spline (-1): not placed",
                            block.line_number,
                            p[1 + o]
                        ));
                        continue;
                    }
                    let line = block.line_number;
                    let opt = |i: usize| p.get(i).and_then(|s| parse_float(s).ok()).unwrap_or(0.0);
                    let (field8, field9, spacing, range, field12) = if v10 {
                        (0.0, 0.0, opt(8 + o), opt(9 + o), 0.0)
                    } else {
                        (
                            opt(8 + o),
                            opt(9 + o),
                            opt(10 + o),
                            opt(11 + o),
                            opt(12 + o),
                        )
                    };
                    let link = if repeater {
                        Some(RepeaterLink {
                            master_tile: parse_i64(p[1], "repeater master tile", line)?,
                            first_instance: parse_i64(p[2], "repeater first instance", line)?
                                .clamp(0, i64::from(u32::MAX))
                                as u32,
                        })
                    } else {
                        None
                    };
                    tile.spline_attachments.push(MapSplineAttachment {
                        sco_file: p[1 + o].to_string(),
                        id_code: parse_u64(p[2 + o], "attachment id", line)?,
                        spline_index: parse_u32(p[3 + o], "attachment spline", line)? as usize,
                        lateral_m: parse_f32(p[4 + o], "attachment lateral", line)?,
                        height_m: parse_f32(p[5 + o], "attachment height", line)?,
                        distance_m: parse_f32(p[6 + o], "attachment distance", line)?,
                        heading_deg: parse_f32(p[7 + o], "attachment heading", line)?,
                        field8,
                        field9,
                        spacing_m: spacing,
                        range_m: range,
                        field12,
                        repeater: link,
                        string_vars: cur.strings(p.get(count_at).copied()),
                    });
                }
                _ => {
                    // Unknown or comment block - ignore gracefully
                }
            }
        }

        Ok(tile)
    }
}

/// A tile block: the keyword with its generic parameters (non-blank, non-comment lines, as
/// [`crate::formats::text::scan_blocks`] gives them) and every line up to the next keyword
/// verbatim (trimmed, blank lines included) for the records that carry string values.
struct TileBlock {
    block: OmsiBlock,
    lines: Vec<String>,
}

impl TileBlock {
    fn cursor(&self) -> LineCursor<'_> {
        LineCursor {
            lines: &self.lines,
            pos: 0,
        }
    }
}

/// Reads a record line by line: positional fields skip blank and comment lines (as the generic
/// scanner does), string values are taken literally, so a blank value stays a blank value instead
/// of pulling the next line (the `Object Nr. N` comment) into its place.
struct LineCursor<'a> {
    lines: &'a [String],
    pos: usize,
}

impl<'a> LineCursor<'a> {
    /// Up to `n` positional fields.
    fn fields(&mut self, n: usize) -> Vec<&'a str> {
        let mut out = Vec::with_capacity(n);
        while out.len() < n && self.pos < self.lines.len() {
            let l = self.lines[self.pos].as_str();
            self.pos += 1;
            if !(l.is_empty() || l.starts_with("//") || l.starts_with('#')) {
                out.push(l);
            }
        }
        out
    }

    /// The `count` string values that follow (fewer when the record is cut short).
    fn strings(&mut self, count: Option<&str>) -> Vec<String> {
        let n = count
            .and_then(|c| c.trim().parse::<usize>().ok())
            .unwrap_or(0);
        let end = (self.pos + n).min(self.lines.len());
        let out = self.lines[self.pos..end].to_vec();
        self.pos = end;
        out
    }
}

/// Splits tile text into [`TileBlock`]s (keyword lines as in `scan_blocks`).
fn scan_tile_blocks(text: &str) -> Vec<TileBlock> {
    let mut out: Vec<TileBlock> = Vec::new();
    for (i, raw_line) in text.lines().enumerate() {
        let line = raw_line.trim_end_matches(['\r', '\n']).trim();
        if !line.is_empty() && raw_line.starts_with('[') && raw_line.trim_end().ends_with(']') {
            if let Some(kw) = parse_keyword(raw_line.trim_end()) {
                out.push(TileBlock {
                    block: OmsiBlock {
                        keyword: kw.to_ascii_lowercase(),
                        line_number: i + 1,
                        parameters: Vec::new(),
                    },
                    lines: Vec::new(),
                });
                continue;
            }
        }
        let Some(current) = out.last_mut() else {
            continue;
        };
        if !(line.is_empty() || line.starts_with("//") || line.starts_with('#')) {
            current.block.parameters.push(line.to_string());
        }
        current.lines.push(line.to_string());
    }
    out
}

fn parse_i64(s: &str, field: &'static str, line: usize) -> Result<i64, MapParseError> {
    s.trim()
        .parse::<i64>()
        .map_err(|_| MapParseError::InvalidNumber {
            field,
            value: s.to_string(),
            line,
        })
}

fn parse_f32(s: &str, field: &'static str, line: usize) -> Result<f32, MapParseError> {
    parse_float(s).map_err(|_| MapParseError::InvalidNumber {
        field,
        value: s.to_string(),
        line,
    })
}

fn parse_i32(s: &str, field: &'static str, line: usize) -> Result<i32, MapParseError> {
    s.trim()
        .parse::<i32>()
        .map_err(|_| MapParseError::InvalidNumber {
            field,
            value: s.to_string(),
            line,
        })
}

fn parse_u32(s: &str, field: &'static str, line: usize) -> Result<u32, MapParseError> {
    s.trim()
        .parse::<u32>()
        .map_err(|_| MapParseError::InvalidNumber {
            field,
            value: s.to_string(),
            line,
        })
}

/// End pose `(x, y, z, heading_deg)` of a spline: straight or circular arc (`radius_m != 0`),
/// the end height rises by `length * (g_start + g_end) / 2`.
fn spline_end_pose(s: &MapSplineInstance) -> (f32, f32, f32, f32) {
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

/// A connection id: `0` means unconnected, and some editors write `-1` for the same thing (all
/// 61 tiles of Neuendorf, Rheinhausen and Ahlheim 5 that failed to parse did).
fn parse_conn(s: &str, field: &'static str, line: usize) -> Result<u64, MapParseError> {
    match s.trim().parse::<i64>() {
        Ok(v) if v < 0 => Ok(0),
        _ => parse_u64(s, field, line),
    }
}

fn parse_u64(s: &str, field: &'static str, line: usize) -> Result<u64, MapParseError> {
    s.trim()
        .parse::<u64>()
        .map_err(|_| MapParseError::InvalidNumber {
            field,
            value: s.to_string(),
            line,
        })
}

fn parse_usize(s: &str, field: &'static str, line: usize) -> Result<usize, MapParseError> {
    s.trim()
        .parse::<usize>()
        .map_err(|_| MapParseError::InvalidNumber {
            field,
            value: s.to_string(),
            line,
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two line modules on a bus-stop sign (`Aachen Linie 33`, tile 1094/10762), one in version 10.
    #[test]
    fn attached_objects_name_their_parent_and_attachment_point() {
        let v14 = "[version]\n14\n\n[attachObj]\n0\nSceneryobjects\\ASEAG\\Haltestelle_Linienmodul.sco\n15988\n15986\n0\n1\n90\n0\n0\n2\n396\nVaals Busstation\n\nObject Nr. 29\n";
        let tile = MapTile::from_text(v14).unwrap();
        let a = &tile.attached_objects[0];
        assert_eq!((a.id_code, a.parent_id, a.attachment), (15988, 15986, 1));
        assert_eq!(a.heading_deg, 90.0);
        assert_eq!((a.pitch_deg, a.roll_deg), (Some(0.0), Some(0.0)));
        assert_eq!(a.string_vars, vec!["396", "Vaals Busstation"]);

        let v10 = "[version]\n10\n\n[attachObj]\n0\nSceneryobjects\\X\\Std_H_Name.sco\n42468\n42465\n0\n3\n180\n1\nSüdermarkt\n\nObject Nr. 264\n";
        let tile = MapTile::from_text(v10).unwrap();
        let a = &tile.attached_objects[0];
        assert_eq!(
            (a.parent_id, a.attachment, a.heading_deg),
            (42465, 3, 180.0)
        );
        assert_eq!((a.pitch_deg, a.roll_deg), (None, None));
        assert_eq!(a.string_vars, vec!["Südermarkt"]);
    }

    /// A blank string value (the `car_park` parking-list selector, Berlin-Spandau) stays blank: the
    /// `Object Nr.` comment after it is not pulled into its place.
    #[test]
    fn blank_string_value_is_kept() {
        let text = "[version]\n14\n\n[object]\n0\nSceneryobjects\\Generic\\car_park.sco\n42\n1\n2\n0\n90\n0\n0\n1\n\n\nObject Nr. 7\n\n[object]\n0\nSceneryobjects\\Generic\\car_park.sco\n43\n1\n2\n0\n90\n0\n0\n6\n\n1\n1\n\n\n\n\nObject Nr. 8\n";
        let tile = MapTile::from_text(text).unwrap();
        assert_eq!(tile.objects.len(), 2);
        assert_eq!(tile.objects[0].string_vars, vec![String::new()]);
        assert_eq!(tile.objects[1].string_vars, vec!["", "1", "1", "", "", ""]);
    }

    /// `[splineAttachement]` and `[splineAttachement_repeater]` (version 14, Berlin-Spandau tile
    /// 2404/11277 style): spacing, range, the repeater's master tile and first instance, strings.
    #[test]
    fn attachment_layout_with_repeater() {
        let text = "[version]\n14\n\n\
[splineAttachement]\n0\nSceneryobjects\\Generic\\car_park.sco\n1001\n2\n4.25\n0.1\n12.5\n90\n0\n-3\n7\n140\n1\n1\n\n\nObject Nr. 5\n\n\
[splineAttachement_repeater]\n0\n17\n5\nSceneryobjects\\Generic\\car_park.sco\n1001\n0\n4.25\n0.1\n12.5\n90\n0\n-3\n7\n140\n1\n1\n1\nObject Nr. 6\n";
        let tile = MapTile::from_text(text).unwrap();
        let [master, rep] = tile.spline_attachments.as_slice() else {
            panic!("two attachments: {:?}", tile.spline_attachments);
        };
        assert_eq!(
            (master.spline_index, master.lateral_m, master.distance_m),
            (2, 4.25, 12.5)
        );
        assert_eq!(
            (
                master.field8,
                master.field9,
                master.spacing_m,
                master.range_m,
                master.field12
            ),
            (0.0, -3.0, 7.0, 140.0, 1.0)
        );
        assert_eq!(master.repeater, None);
        assert_eq!(master.string_vars, vec![String::new()]);
        assert_eq!(master.instances(), 0..=20);
        assert!((master.chainage(3) - 33.5).abs() < 1e-9);
        assert_eq!(
            rep.repeater,
            Some(RepeaterLink {
                master_tile: 17,
                first_instance: 5
            })
        );
        assert_eq!(rep.spline_index, 0);
        assert_eq!(rep.instances(), 5..=20);
        assert_eq!(rep.string_vars, vec!["1"]);
    }

    /// Version 10 attachments have no `field8`, `field9` and `field12` (Neuendorf `tile_0_-3.map`).
    #[test]
    fn version_10_attachment_layout() {
        let text = "[version]\n10\n\n[splineAttachement]\n0\nSceneryobjects\\Generic\\car_park.sco\n8852\n2\n-4.37\n0.15\n8.45\n180\n5.5\n20\n4\nTree_Large_03.tga\n28.400\n1.073\n\n\nObject Nr. 155\n";
        let tile = MapTile::from_text(text).unwrap();
        let a = &tile.spline_attachments[0];
        assert_eq!((a.spacing_m, a.range_m), (5.5, 20.0));
        assert_eq!(a.instances(), 0..=3);
        assert_eq!(
            a.string_vars,
            vec!["Tree_Large_03.tga", "28.400", "1.073", ""]
        );
    }

    /// A single attachment: no spacing or range.
    #[test]
    fn single_attachment_has_one_instance() {
        let text =
            "[version]\n14\n\n[splineAttachement]\n0\na.sco\n1\n0\n0\n0\n10\n0\n0\n0\n0\n0\n0\n0\n";
        let tile = MapTile::from_text(text).unwrap();
        assert_eq!(tile.spline_attachments[0].instances(), 0..=0);
        assert!(!tile.spline_attachments[0].is_run());
    }

    /// Neuenbreid `tile_9_-19.map` (attachments without a carrier) and `tile_3_11.map` (a record cut
    /// off by NUL bytes): left out, but named in `warnings`.
    #[test]
    fn records_that_cannot_be_placed_are_reported() {
        let text = "[version]\n14\n\n[splineAttachement]\n0\nSceneryobjects\\Trees_MC\\shrubbery_04_05.sco\n855395\n-1\n2.0\n-0.5\n1.7\n90\n0\n0\n\n[object]\n0\nSceneryobjects\\Trees_MC\\fir\n";
        let tile = MapTile::from_text(text).unwrap();
        assert!(tile.spline_attachments.is_empty());
        assert!(tile.objects.is_empty());
        assert_eq!(tile.warnings.len(), 2, "{:?}", tile.warnings);
        assert!(tile.warnings[0].contains("names no spline"));
        assert!(tile.warnings[1].contains("[object]"));
    }

    #[test]
    fn test_parse_minimal_tile() {
        let text = r#"
File created with test

[version]
14

[terrain]

[object]
0
Sceneryobjects\Example\house.sco
100001
150.0
120.0
5.0
90.0
0.0
0.0
0

[spline]
0
Splines\Example\road.sli
200001
0
0
10.0
2.5
20.0
180.0
50.0
0.0
0
0
0
0
0.0
0.0
0
"#;
        let tile = MapTile::from_text(text).unwrap();
        assert_eq!(tile.version, Some(14));
        assert!(tile.has_terrain);

        assert_eq!(tile.objects.len(), 1);
        let obj = &tile.objects[0];
        assert_eq!(obj.sco_file, r"Sceneryobjects\Example\house.sco");
        assert_eq!(obj.id_code, 100001);
        assert_eq!(obj.position, Vec3::new(150.0, 120.0, 5.0));
        assert_eq!(obj.heading_deg, 90.0);
        assert_eq!(obj.pitch_deg, Some(0.0));
        assert_eq!(obj.roll_deg, Some(0.0));

        assert_eq!(tile.splines.len(), 1);
        let spl = &tile.splines[0];
        assert_eq!(spl.sli_file, r"Splines\Example\road.sli");
        assert_eq!(spl.id_code, 200001);
        // Position order: Line 5 (10.0) is X, Line 6 (2.5) is Z (elevation), Line 7 (20.0) is Y!
        assert_eq!(spl.position, Vec3::new(10.0, 20.0, 2.5));
        assert_eq!(spl.heading_deg, 180.0);
        assert_eq!(spl.length_m, 50.0);
        assert_eq!(spl.radius_m, 0.0);
    }

    /// MI-0021: a version 10 tile has one connection field; `-1` places the spline, `0` continues
    /// the previous one (all pose fields are zero).
    #[test]
    fn test_version_10_splines_are_placed_or_continue_the_previous() {
        let rec = |id: u32, conn: &str, x: &str, y: &str, h: &str, len: &str| {
            format!(
                "[spline]\n0\nSplines\\a.sli\n{id}\n{conn}\n{x}\n0\n{y}\n{h}\n{len}\n0\n0\n0\n0\n0\n\n"
            )
        };
        let text = format!(
            "File created\n\n[version]\n10\n\n{}{}",
            rec(1, "-1", "100", "200", "90", "30"),
            rec(2, "0", "0", "0", "0", "10")
        )
            + "[rule]\n0\ntrafficdensity\n3.000\n\n[rule]\n1\nspeedlimit\n30\nObject Nr. 0\n";
        let tile = MapTile::from_text(&text).expect("v10 tile parses");
        assert_eq!(tile.rules.len(), 2, "v10 rules have no flag line");
        assert_eq!(tile.rules[1].flag, 0);
        assert_eq!(tile.splines.len(), 2);
        assert_eq!(tile.splines[0].position.x, 100.0);
        assert_eq!(tile.splines[0].position.y, 200.0);
        assert_eq!(tile.splines[0].length_m, 30.0);
        // heading 90 degrees runs towards +x: the second spline starts 30 m east of the first
        let second = &tile.splines[1];
        assert!(
            (second.position.x - 130.0).abs() < 1e-3,
            "{:?}",
            second.position
        );
        assert!((second.position.y - 200.0).abs() < 1e-3);
        assert!((second.heading_deg - 90.0).abs() < 1e-3);
        assert_eq!(second.length_m, 10.0);
    }

    #[test]
    fn test_spline_connection_minus_one_means_unconnected() {
        let text = "[version]\n14\n\n[spline]\n0\nSplines\\a.sli\n77\n-1\n-1\n1.0\n2.0\n3.0\n0.0\n10.0\n0.0\n";
        let tile = MapTile::from_text(text).expect("-1 connections parse");
        assert_eq!(tile.splines.len(), 1);
        assert_eq!(tile.splines[0].conn_id_1, 0);
        assert_eq!(tile.splines[0].conn_id_2, 0);
    }
}
