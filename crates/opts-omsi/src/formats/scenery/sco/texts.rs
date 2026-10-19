//! Text textures, crossing-traffic controllers, scripts, variable lists, sounds and the passenger cabin.

use super::*;

/// Applies the block if its keyword is one of this module's; says whether it was.
pub(super) fn apply(obj: &mut SceneryObject, block: &OmsiBlock) -> Result<bool, SceneryParseError> {
    match block.keyword.as_str() {
        "texttexture" | "texttexture_enh" => {
            if block.parameters.len() >= 8 {
                // A number indexes a string variable; anything else names one.
                let (s_idx, s_var) = match block.parameters[0].trim().parse::<usize>() {
                    Ok(i) => (i, None),
                    Err(_) => (0, Some(block.parameters[0].trim().to_owned())),
                };
                let font = block.parameters[1].clone();
                let w = parse_u32(&block.parameters[2], "texttexture width", block.line_number)?;
                let h = parse_u32(
                    &block.parameters[3],
                    "texttexture height",
                    block.line_number,
                )?;
                let mono = parse_u32(&block.parameters[4], "texttexture mono", block.line_number)?;
                let r = parse_u8(&block.parameters[5], "texttexture r", block.line_number)?;
                let g = parse_u8(&block.parameters[6], "texttexture g", block.line_number)?;
                let b = parse_u8(&block.parameters[7], "texttexture b", block.line_number)?;
                obj.text_textures.push(ScoTextTexture {
                    string_index: s_idx,
                    string_var: s_var,
                    font_name: font,
                    width: w,
                    height: h,
                    color_mode: mono,
                    text_color: [r, g, b],
                    orientation: block
                        .parameters
                        .get(8)
                        .and_then(|v| v.trim().parse().ok())
                        .unwrap_or(0),
                });
            }
        }
        "ctc" => {
            if block.parameters.len() >= 3 {
                let var = block.parameters[0].clone();
                let dir = block.parameters[1].clone();
                let idx = parse_usize(&block.parameters[2], "CTC index", block.line_number)?;
                obj.ctc = Some(ScoCtc {
                    var_name: var,
                    texture_dir: dir,
                    default_index: idx,
                });
            }
        }
        "ctctexture" => {
            if block.parameters.len() >= 2 {
                let name = block.parameters[0].clone();
                let file = block.parameters[1].clone();
                obj.ctc_textures.push((name, file));
            }
        }
        "script" => {
            parse_counted_list(
                &block.parameters,
                &mut obj.scripts,
                "script",
                block.line_number,
            )?;
        }
        "varnamelist" => {
            parse_counted_list(
                &block.parameters,
                &mut obj.varnamelists,
                "varnamelist",
                block.line_number,
            )?;
        }
        "stringvarnamelist" => {
            parse_counted_list(
                &block.parameters,
                &mut obj.stringvarnamelists,
                "stringvarnamelist",
                block.line_number,
            )?;
        }
        "constfile" => {
            parse_counted_list(
                &block.parameters,
                &mut obj.constfiles,
                "constfile",
                block.line_number,
            )?;
        }
        "sound" => {
            if let Some(val) = block.parameters.first() {
                obj.sound = Some(val.clone());
            }
        }
        "passengercabin" => {
            if let Some(val) = block.parameters.first() {
                obj.passenger_cabin = Some(val.clone());
            }
        }
        _ => return Ok(false),
    }
    Ok(true)
}
