//! Block parser for OMSI map tiles (`tile_<x>_<y>.map`).

use super::attachment::{parse_attached_object, parse_spline_attachment};
use super::scan::{
    parse_conn, parse_f32, parse_i32, parse_u32, parse_u64, parse_usize, scan_tile_blocks,
    TileBlock,
};
use super::*;

mod objects;
mod rules;
mod splines;

use objects::parse_object;
use rules::parse_rule;
use splines::parse_spline;

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
