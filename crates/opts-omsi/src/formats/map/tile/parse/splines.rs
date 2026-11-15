//! Spline blocks of an OMSI map tile.

use super::*;

pub(super) fn parse_spline_block(
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

pub(super) fn parse_spline(
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
