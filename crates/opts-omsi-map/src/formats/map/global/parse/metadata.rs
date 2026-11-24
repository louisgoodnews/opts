//! The metadata block of `global.cfg`.

use super::*;

pub(super) fn parse_metadata_block(
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
