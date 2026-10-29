//! Block parser for OMSI map tiles (`tile_<x>_<y>.map`).

use super::attachment::{parse_attached_object, parse_spline_attachment};
use super::scan::{
    parse_conn, parse_f32, parse_i32, parse_u32, parse_u64, parse_usize, scan_tile_blocks,
    TileBlock,
};
use super::*;

fn parse_object(
    raw: &TileBlock,
    tile: &mut MapTile,
    rule_owner: &mut Option<RuleOwner>,
    prev_object: &mut Option<usize>,
) -> Result<(), MapParseError> {
    let block = &raw.block;
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
        return Ok(());
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
    *rule_owner = Some(RuleOwner::Object {
        index: tile.objects.len(),
        id_code: id,
    });
    *prev_object = Some(tile.objects.len());
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
    Ok(())
}

fn parse_spline_block(
    block: &OmsiBlock,
    tile: &mut MapTile,
    chained_v10: bool,
    rule_owner: &mut Option<RuleOwner>,
) -> Result<(), MapParseError> {
    let has_h = block.keyword == "spline_h";
    let flag = parse_i32(&block.parameters[0], "spline flag", block.line_number)?;
    let sli = block.parameters[1].clone();
    let id = parse_u64(&block.parameters[2], "spline id", block.line_number)?;
    let c1 = parse_conn(&block.parameters[3], "spline conn1", block.line_number)?;
    let c2 = parse_conn(&block.parameters[4], "spline conn2", block.line_number)?;
    let px = parse_f32(&block.parameters[5], "spline x", block.line_number)?;
    let pz = parse_f32(
        &block.parameters[6],
        "spline elevation z",
        block.line_number,
    )?;
    let py = parse_f32(&block.parameters[7], "spline y", block.line_number)?;
    let heading = parse_f32(&block.parameters[8], "spline heading", block.line_number)?;
    let length = parse_f32(&block.parameters[9], "spline length", block.line_number)?;
    let radius = parse_f32(&block.parameters[10], "spline radius", block.line_number)?;

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
            tile.warnings.push(format!(
                "line {}: version 10 [spline] {} continues no spline: skipped",
                block.line_number, sli
            ));
            return Ok(());
        };
        let end = spline_end_pose(prev);
        (px, py, pz, heading) = (end.0, end.1, end.2, end.3);
    }
    *rule_owner = Some(RuleOwner::Spline {
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
    Ok(())
}

fn parse_spline(
    raw: &TileBlock,
    tile: &mut MapTile,
    rule_owner: &mut Option<RuleOwner>,
) -> Result<(), MapParseError> {
    let block = &raw.block;
    let is_v10 = tile.version == Some(10);
    let chained_v10 = is_v10 && block.parameters.get(3).is_some_and(|c| c.trim() != "-1");
    let normalised: Vec<String> = if is_v10 && block.parameters.len() >= 4 {
        let mut v = block.parameters.clone();
        v.insert(4, "0".to_string());
        v
    } else {
        block.parameters.clone()
    };
    let block = OmsiBlock {
        keyword: block.keyword.clone(),
        line_number: block.line_number,
        parameters: normalised,
    };
    if block.parameters.len() >= 11 {
        parse_spline_block(&block, tile, chained_v10, rule_owner)?;
    } else {
        tile.warnings.push(format!(
            "line {}: [{}] with {} fields skipped (damaged record)",
            block.line_number,
            block.keyword,
            block.parameters.len()
        ));
    }
    Ok(())
}

fn parse_rule(
    raw: &TileBlock,
    tile: &mut MapTile,
    rule_owner: Option<RuleOwner>,
) -> Result<(), MapParseError> {
    let block = &raw.block;
    let v10_rule = tile.version == Some(10);
    if block.parameters.len() >= 4 || (v10_rule && block.parameters.len() >= 3) {
        let idx = parse_usize(&block.parameters[0], "rule path_index", block.line_number)?;
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
    Ok(())
}

impl MapTile {
    /// Parses a map tile from decoded text.
    ///
    /// # Errors
    /// Returns [`MapParseError`] if required parameters are missing or malformed.
    pub fn from_text(text: &str) -> Result<Self, MapParseError> {
        let blocks = scan_tile_blocks(text);
        let mut tile = MapTile::default();
        let mut rule_owner: Option<RuleOwner> = None;
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
                "terrain" => tile.has_terrain = true,
                "water" => tile.has_water = true,
                "variable_terrain" => tile.has_variable_terrain = true,
                "variable_terrainlightmap" => tile.has_variable_terrain_lightmap = true,
                "spline_terrain_align" | "spline_terrain_align_2" => tile.mark_aligned(block),
                "object" => parse_object(raw, &mut tile, &mut rule_owner, &mut prev_object)?,
                "spline" | "spline_h" => parse_spline(raw, &mut tile, &mut rule_owner)?,
                "rule" => parse_rule(raw, &mut tile, rule_owner)?,
                "attachobj" => parse_attached_object(raw, &mut tile)?,
                "splineattachement" | "splineattachement_repeater" => {
                    parse_spline_attachment(raw, &mut tile)?;
                }
                _ => {}
            }
        }

        Ok(tile)
    }
}
