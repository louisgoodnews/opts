//! The environment and traffic density blocks of `global.cfg`.

use super::*;

pub(super) fn parse_environment_block(
    block: &OmsiBlock,
    manifest: &mut MapManifest,
) -> Result<(), MapParseError> {
    match block.keyword.as_str() {
        "years" if block.parameters.len() >= 2 => {
            let y0 = parse_i32(&block.parameters[0], "years start", block.line_number)?;
            let y1 = parse_i32(&block.parameters[1], "years end", block.line_number)?;
            manifest.years = Some((y0, y1));
        }
        "realyearoffset" => {
            if let Some(val) = block.parameters.first() {
                manifest.real_year_offset =
                    Some(parse_i32(val, "realyearoffset", block.line_number)?);
            }
        }
        "repair_time_min" => {
            if let Some(val) = block.parameters.first() {
                manifest.repair_time_min =
                    Some(parse_f32(val, "repair_time_min", block.line_number)?);
            }
        }
        "worldcoordinates" => manifest.world_coordinates = true,
        "realrail" => manifest.real_rail = true,
        "dynhelperactive" => manifest.dyn_helper_active = true,
        "map" if block.parameters.len() >= 3 => {
            let x = parse_i32(&block.parameters[0], "map tile x", block.line_number)?;
            let y = parse_i32(&block.parameters[1], "map tile y", block.line_number)?;
            manifest.tiles.push(MapTileEntry {
                x,
                y,
                file_name: block.parameters[2].clone(),
            });
        }
        _ => {}
    }
    Ok(())
}

pub(super) fn parse_traffic_density_block(
    block: &OmsiBlock,
    manifest: &mut MapManifest,
) -> Result<(), MapParseError> {
    match block.keyword.as_str() {
        "addseason" if block.parameters.len() >= 3 => {
            let s_id = parse_u32(&block.parameters[0], "addseason id", block.line_number)?;
            let start = parse_u32(&block.parameters[1], "addseason start", block.line_number)?;
            let end = parse_u32(&block.parameters[2], "addseason end", block.line_number)?;
            manifest.seasons.push(MapSeason {
                season_id: s_id,
                start_day: start,
                end_day: end,
            });
        }
        "trafficdensity_road" if block.parameters.len() >= 2 => {
            let h = parse_f32(
                &block.parameters[0],
                "trafficdensity_road hour",
                block.line_number,
            )?;
            let f = parse_f32(
                &block.parameters[1],
                "trafficdensity_road factor",
                block.line_number,
            )?;
            manifest.traffic_density_road.push((h, f));
        }
        "trafficdensity_passenger" if block.parameters.len() >= 2 => {
            let h = parse_f32(
                &block.parameters[0],
                "trafficdensity_passenger hour",
                block.line_number,
            )?;
            let f = parse_f32(
                &block.parameters[1],
                "trafficdensity_passenger factor",
                block.line_number,
            )?;
            manifest.traffic_density_passenger.push((h, f));
        }
        _ => {}
    }
    Ok(())
}
