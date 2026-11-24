//! Parser for tile object attachments and spline attachments.

use super::scan::{parse_f32, parse_i32, parse_i64, parse_u32, parse_u64, TileBlock};
use super::{MapAttachedObject, MapSplineAttachment, MapTile, RepeaterLink};
use crate::formats::map::error::MapParseError;
use crate::formats::text::parse_float;

pub(super) fn parse_attached_object(
    raw: &TileBlock,
    tile: &mut MapTile,
) -> Result<(), MapParseError> {
    let block = &raw.block;
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
        return Ok(());
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
    Ok(())
}

pub(super) fn parse_spline_attachment(
    raw: &TileBlock,
    tile: &mut MapTile,
) -> Result<(), MapParseError> {
    let block = &raw.block;
    let repeater = block.keyword == "splineattachement_repeater";
    let o = if repeater { 2 } else { 0 };
    let v10 = tile.version == Some(10);
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
        return Ok(());
    }
    if p[3 + o].trim() == "-1" {
        tile.warnings.push(format!(
            "line {}: attachment {} names no spline (-1): not placed",
            block.line_number,
            p[1 + o]
        ));
        return Ok(());
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
                .clamp(0, i64::from(u32::MAX)) as u32,
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
    Ok(())
}
