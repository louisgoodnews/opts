//! Parser and data structures for OMSI map tile files (`tile_<x>_<y>.map`).
//!
//! A map tile is a 300 m × 300 m world grid square containing placed scenery
//! objects (`[object]`), splines (`[spline]`, `[spline_h]`), traffic rules (`[rule]`),
//! and spline attachments.
//!
//! See specification in `docs/src/formats/map/tiles.md`.

use glam::Vec3;

use super::error::MapParseError;
use crate::formats::text::{decode_bytes, parse_float, scan_blocks};

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

/// Object attached to a spline (`[splineAttachement]`) or repeated along it
/// (`[splineAttachement_repeater]`): street furniture, fences, shrubs, parking bays.
///
/// `distance_m` is a chainage: for single attachments it is measured from the start of the
/// connected spline chain that `spline_index` belongs to (walk `conn_id_1` backwards); for
/// repeaters it is measured from the start of that spline itself.
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
    /// Chainage of the (first) instance in metres.
    pub distance_m: f32,
    /// Heading relative to the spline direction in degrees.
    pub heading_deg: f32,
    /// Repetition `(span in metres, count)` for `[splineAttachement_repeater]`.
    pub repeat: Option<(f32, u32)>,
    /// Remaining raw parameters (visibility limits and unidentified fields).
    pub raw_params: Vec<String>,
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
        let blocks = scan_blocks(text);
        let mut tile = MapTile::default();
        // The spline or object a following `[rule]` belongs to.
        let mut rule_owner: Option<RuleOwner> = None;
        // The object a `[varparent]` right after it belongs to.
        let mut prev_object: Option<usize> = None;

        for block in &blocks {
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
                    if block.parameters.len() >= 7 {
                        let flag =
                            parse_i32(&block.parameters[0], "object flag", block.line_number)?;
                        let sco = block.parameters[1].clone();
                        let id = parse_u64(&block.parameters[2], "object id", block.line_number)?;
                        let x = parse_f32(&block.parameters[3], "object x", block.line_number)?;
                        let y = parse_f32(&block.parameters[4], "object y", block.line_number)?;
                        let z = parse_f32(&block.parameters[5], "object z", block.line_number)?;
                        let heading =
                            parse_f32(&block.parameters[6], "object heading", block.line_number)?;

                        let is_v10 = tile.version == Some(10);
                        let (pitch, roll, string_vars) = if is_v10 {
                            let n_strings = block
                                .parameters
                                .get(7)
                                .and_then(|s| s.parse::<usize>().ok())
                                .unwrap_or(0);
                            let strings = block
                                .parameters
                                .iter()
                                .skip(8)
                                .take(n_strings)
                                .cloned()
                                .collect();
                            (None, None, strings)
                        } else {
                            let p = block.parameters.get(7).and_then(|s| parse_float(s).ok());
                            let r = block.parameters.get(8).and_then(|s| parse_float(s).ok());
                            let n_strings = block
                                .parameters
                                .get(9)
                                .and_then(|s| s.parse::<usize>().ok())
                                .unwrap_or(0);
                            let strings = block
                                .parameters
                                .iter()
                                .skip(10)
                                .take(n_strings)
                                .cloned()
                                .collect();
                            (p, r, strings)
                        };

                        rule_owner = Some(RuleOwner::Object {
                            index: tile.objects.len(),
                            id_code: id,
                        });
                        prev_object = Some(tile.objects.len());
                        tile.objects.push(MapObjectInstance {
                            flag,
                            sco_file: sco,
                            id_code: id,
                            position: Vec3::new(x, y, z),
                            heading_deg: heading,
                            pitch_deg: pitch,
                            roll_deg: roll,
                            string_vars,
                        });
                    }
                }
                "spline" | "spline_h" => {
                    if block.parameters.len() >= 11 {
                        let has_h = block.keyword == "spline_h";
                        let flag =
                            parse_i32(&block.parameters[0], "spline flag", block.line_number)?;
                        let sli = block.parameters[1].clone();
                        let id = parse_u64(&block.parameters[2], "spline id", block.line_number)?;
                        let c1 =
                            parse_u64(&block.parameters[3], "spline conn1", block.line_number)?;
                        let c2 =
                            parse_u64(&block.parameters[4], "spline conn2", block.line_number)?;
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
                    }
                }
                "rule" => {
                    if block.parameters.len() >= 4 {
                        let idx = parse_usize(
                            &block.parameters[0],
                            "rule path_index",
                            block.line_number,
                        )?;
                        let rtype = block.parameters[1].clone();
                        let val = parse_f32(&block.parameters[2], "rule value", block.line_number)?;
                        let flag = parse_u32(&block.parameters[3], "rule flag", block.line_number)?;
                        tile.rules.push(MapTrafficRule {
                            owner: rule_owner,
                            path_index: idx,
                            rule_type: rtype,
                            value: val,
                            flag,
                        });
                    }
                }
                "splineattachement" | "splineattachement_repeater" => {
                    let repeater = block.keyword == "splineattachement_repeater";
                    let o = if repeater { 2 } else { 0 };
                    if block.parameters.len() < 8 + o {
                        continue;
                    }
                    let p = &block.parameters;
                    // `-1` as carrier: the attachment belongs to no spline (an editor leftover, e.g.
                    // Berlin-Spandau tile 2394/11275). It has no place to be put.
                    if p[3 + o].trim() == "-1" {
                        continue;
                    }
                    let repeat = if repeater {
                        Some((
                            parse_f32(&p[1], "attachment span", block.line_number)?,
                            parse_u32(&p[2], "attachment count", block.line_number)?,
                        ))
                    } else {
                        None
                    };
                    tile.spline_attachments.push(MapSplineAttachment {
                        sco_file: p[1 + o].clone(),
                        id_code: parse_u64(&p[2 + o], "attachment id", block.line_number)?,
                        spline_index: parse_u32(&p[3 + o], "attachment spline", block.line_number)?
                            as usize,
                        lateral_m: parse_f32(&p[4 + o], "attachment lateral", block.line_number)?,
                        height_m: parse_f32(&p[5 + o], "attachment height", block.line_number)?,
                        distance_m: parse_f32(&p[6 + o], "attachment distance", block.line_number)?,
                        heading_deg: parse_f32(&p[7 + o], "attachment heading", block.line_number)?,
                        repeat,
                        raw_params: p[8 + o..].to_vec(),
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
}
