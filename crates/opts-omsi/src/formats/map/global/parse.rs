//! Parser implementation for OMSI map scenario manifests (`global.cfg`).

use glam::Vec3;

use super::{MapEntryPoint, MapGroundTex, MapManifest, MapSeason, MapTileEntry};
use crate::formats::map::error::MapParseError;
use crate::formats::text::{parse_float, scan_blocks, OmsiBlock};

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

fn parse_usize(s: &str, field: &'static str, line: usize) -> Result<usize, MapParseError> {
    s.trim()
        .parse::<usize>()
        .map_err(|_| MapParseError::InvalidNumber {
            field,
            value: s.to_string(),
            line,
        })
}

/// The lines after the keyword line `keyword_line` (1-based) up to the next keyword line, trimmed
/// but otherwise untouched: blank lines and lines starting with `#` or `//` are kept.
fn raw_block_lines(text: &str, keyword_line: usize) -> Vec<String> {
    let mut out = Vec::new();
    for raw in text.lines().skip(keyword_line) {
        let t = raw.trim();
        if raw.starts_with('[') && raw.trim_end().ends_with(']') {
            break;
        }
        out.push(t.to_string());
    }
    while out.first().is_some_and(String::is_empty) {
        out.remove(0);
    }
    out
}

fn parse_entrypoints_block(
    text: &str,
    block: &OmsiBlock,
    manifest: &mut MapManifest,
) -> Result<(), MapParseError> {
    let raw = raw_block_lines(text, block.line_number);
    let Some(first) = raw.first() else {
        return Ok(());
    };
    let count = parse_usize(first, "entrypoints count", block.line_number)?;
    let records = &raw[1..];
    for chunk in records.chunks_exact(12).take(count) {
        let id = parse_u32(&chunk[0], "entrypoint id", block.line_number)?;
        let id_code = parse_u32(&chunk[1], "entrypoint id_code", block.line_number)?;
        let px = parse_f32(&chunk[3], "entrypoint px", block.line_number)?;
        let py = parse_f32(&chunk[4], "entrypoint py", block.line_number)?;
        let pz = parse_f32(&chunk[5], "entrypoint pz", block.line_number)?;
        let dx = parse_f32(&chunk[7], "entrypoint dx", block.line_number)?;
        let dy = parse_f32(&chunk[8], "entrypoint dy", block.line_number)?;
        let dz = parse_f32(&chunk[9], "entrypoint dz", block.line_number)?;
        let name = chunk[11].clone();

        manifest.entry_points.push(MapEntryPoint {
            id,
            id_code,
            position: Vec3::new(px, py, pz),
            direction: Vec3::new(dx, dy, dz),
            name,
        });
    }
    Ok(())
}

fn parse_groundtex_block(
    block: &OmsiBlock,
    manifest: &mut MapManifest,
) -> Result<(), MapParseError> {
    if block.parameters.len() >= 5 {
        let gtex = block.parameters[0].clone();
        let dtex = block.parameters[1].clone();
        let p0 = parse_f32(&block.parameters[2], "groundtex p0", block.line_number)?;
        let p1 = parse_f32(&block.parameters[3], "groundtex p1", block.line_number)?;
        let p2 = parse_f32(&block.parameters[4], "groundtex p2", block.line_number)?;
        manifest.ground_textures.push(MapGroundTex {
            ground_texture: gtex,
            detail_texture: dtex,
            params: [p0, p1, p2],
        });
    }
    Ok(())
}

fn parse_metadata_block(
    block: &OmsiBlock,
    manifest: &mut MapManifest,
) -> Result<(), MapParseError> {
    match block.keyword.as_str() {
        "name" => {
            if let Some(val) = block.parameters.first() {
                manifest.name = val.clone();
            }
        }
        "friendlyname" => {
            if let Some(val) = block.parameters.first() {
                manifest.friendly_name = Some(val.clone());
            }
        }
        "description" => {
            manifest.description = block.parameters.clone();
        }
        "version" => {
            if let Some(val) = block.parameters.first() {
                manifest.version = Some(parse_u32(val, "version", block.line_number)?);
            }
        }
        "nextidcode" => {
            if let Some(val) = block.parameters.first() {
                manifest.next_id_code = Some(parse_u32(val, "NextIDCode", block.line_number)?);
            }
        }
        "moneysystem" => {
            if let Some(val) = block.parameters.first() {
                manifest.money_system = Some(val.clone());
            }
        }
        "ticketpack" => {
            if let Some(val) = block.parameters.first() {
                manifest.ticket_pack = Some(val.clone());
            }
        }
        "standarddepot" => {
            if let Some(val) = block.parameters.first() {
                manifest.standard_depot = Some(val.clone());
            }
        }
        _ => {}
    }
    Ok(())
}

fn parse_environment_block(
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

fn parse_traffic_density_block(
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

pub(crate) fn parse_manifest(text: &str) -> Result<MapManifest, MapParseError> {
    let blocks = scan_blocks(text);
    let mut manifest = MapManifest::default();

    for block in &blocks {
        match block.keyword.as_str() {
            "name" | "friendlyname" | "description" | "version" | "nextidcode" | "moneysystem"
            | "ticketpack" | "standarddepot" => {
                parse_metadata_block(block, &mut manifest)?;
            }
            "years" | "realyearoffset" | "repair_time_min" | "worldcoordinates" | "realrail"
            | "dynhelperactive" | "map" => {
                parse_environment_block(block, &mut manifest)?;
            }
            "entrypoints" => {
                parse_entrypoints_block(text, block, &mut manifest)?;
            }
            "groundtex" => {
                parse_groundtex_block(block, &mut manifest)?;
            }
            "addseason" | "trafficdensity_road" | "trafficdensity_passenger" => {
                parse_traffic_density_block(block, &mut manifest)?;
            }
            _ => {}
        }
    }

    Ok(manifest)
}
