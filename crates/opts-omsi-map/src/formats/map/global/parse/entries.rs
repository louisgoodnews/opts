//! The entry point and ground texture blocks of `global.cfg`.

use super::*;

pub(super) fn parse_entrypoints_block(
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

pub(super) fn parse_groundtex_block(
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
