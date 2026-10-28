//! Block parser for OMSI map tiles (`tile_<x>_<y>.map`).

use super::*;
use scan::{
    parse_conn, parse_f32, parse_i32, parse_i64, parse_u32, parse_u64, parse_usize,
    scan_tile_blocks,
};

impl MapTile {
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
                "spline_terrain_align" | "spline_terrain_align_2" => tile.mark_aligned(block),
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
