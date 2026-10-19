//! Global directives of a model file (the lines outside any mesh block).

use super::*;

pub(super) fn handle_global_directive(
    config: &mut ModelConfig,
    block: &OmsiBlock,
) -> Result<(), ModelParseError> {
    match block.keyword.as_str() {
        "tex_detail_factor" => {
            let params = require_params(block, 1)?;
            config.tex_detail_factor = Some(parse_number::<f32>(&params[0], block.line_number)?);
        }
        "ctc" => {
            // The default-index line is absent in some human manifests (e.g. Busfahrer_Bremen).
            let params = require_params(block, 2)?;
            let default_index = match params.get(2) {
                Some(p) => parse_number::<i32>(p, block.line_number)?,
                None => 0,
            };
            config.ctc_schemes.push(CtcScheme {
                name: params[0].clone(),
                texture_path: params[1].clone(),
                default_index,
            });
        }
        "ctctexture" => {
            let params = require_params(block, 2)?;
            config.ctc_textures.push(CtcTexture {
                name: params[0].clone(),
                texture_path: params[1].clone(),
            });
        }
        "texttexture" => {
            let params = require_params(block, 8)?;
            let width = parse_number::<u32>(&params[2], block.line_number)?;
            let height = parse_number::<u32>(&params[3], block.line_number)?;
            let full_color = parse_number::<u32>(&params[4], block.line_number)? != 0;
            let r = parse_number::<u8>(&params[5], block.line_number)?;
            let g = parse_number::<u8>(&params[6], block.line_number)?;
            let b = parse_number::<u8>(&params[7], block.line_number)?;
            config.text_textures.push(TextTexture {
                variable: params[0].clone(),
                font_name: params[1].clone(),
                width,
                height,
                full_color,
                color_rgb: [r, g, b],
                orientation: None,
                grid: None,
            });
        }
        "texttexture_enh" => {
            let params = require_params(block, 10)?;
            let width = parse_number::<u32>(&params[2], block.line_number)?;
            let height = parse_number::<u32>(&params[3], block.line_number)?;
            let full_color = parse_number::<u32>(&params[4], block.line_number)? != 0;
            let r = parse_number::<u8>(&params[5], block.line_number)?;
            let g = parse_number::<u8>(&params[6], block.line_number)?;
            let b = parse_number::<u8>(&params[7], block.line_number)?;
            let orientation = Some(parse_number::<u32>(&params[8], block.line_number)?);
            let grid = Some(parse_number::<u32>(&params[9], block.line_number)?);
            config.text_textures.push(TextTexture {
                variable: params[0].clone(),
                font_name: params[1].clone(),
                width,
                height,
                full_color,
                color_rgb: [r, g, b],
                orientation,
                grid,
            });
        }
        "scripttexture" => {
            let params = require_params(block, 2)?;
            let width = parse_number::<u32>(&params[0], block.line_number)?;
            let height = parse_number::<u32>(&params[1], block.line_number)?;
            config.script_textures.push(ScriptTexture { width, height });
        }
        _ => {}
    }
    Ok(())
}
