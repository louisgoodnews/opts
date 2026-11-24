//! Parser implementation for OMSI map scenario manifests (`global.cfg`).

use glam::Vec3;

use super::{MapEntryPoint, MapGroundTex, MapManifest, MapSeason, MapTileEntry};
use crate::formats::map::error::MapParseError;
use crate::formats::text::{parse_float, scan_blocks, OmsiBlock};

mod entries;
mod environment;
mod metadata;
mod scalars;

use entries::{parse_entrypoints_block, parse_groundtex_block};
use environment::{parse_environment_block, parse_traffic_density_block};
use metadata::parse_metadata_block;
use scalars::{parse_f32, parse_i32, parse_u32, parse_usize, raw_block_lines};

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
