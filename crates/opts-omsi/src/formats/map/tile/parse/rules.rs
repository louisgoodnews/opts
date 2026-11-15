//! Traffic rule blocks of an OMSI map tile.

use super::*;

pub(super) fn parse_rule(
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
