//! Object blocks of an OMSI map tile.

use super::*;

pub(super) fn parse_object(
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
