//! Text textures, crossing-traffic controllers, scripts, variable lists, sounds and the passenger cabin.

use super::*;

fn parse_text_texture(
    params: &[String],
    line_number: usize,
) -> Result<Option<ScoTextTexture>, SceneryParseError> {
    if params.len() < 8 {
        return Ok(None);
    }
    let (s_idx, s_var) = match params[0].trim().parse::<usize>() {
        Ok(i) => (i, None),
        Err(_) => (0, Some(params[0].trim().to_owned())),
    };
    let font = params[1].clone();
    let w = parse_u32(&params[2], "texttexture width", line_number)?;
    let h = parse_u32(&params[3], "texttexture height", line_number)?;
    let mono = parse_u32(&params[4], "texttexture mono", line_number)?;
    let r = parse_u8(&params[5], "texttexture r", line_number)?;
    let g = parse_u8(&params[6], "texttexture g", line_number)?;
    let b = parse_u8(&params[7], "texttexture b", line_number)?;
    let orientation = params
        .get(8)
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(0);
    Ok(Some(ScoTextTexture {
        string_index: s_idx,
        string_var: s_var,
        font_name: font,
        width: w,
        height: h,
        color_mode: mono,
        text_color: [r, g, b],
        orientation,
    }))
}

fn parse_ctc(params: &[String], line_number: usize) -> Result<Option<ScoCtc>, SceneryParseError> {
    if params.len() < 3 {
        return Ok(None);
    }
    let var = params[0].clone();
    let dir = params[1].clone();
    let idx = parse_usize(&params[2], "CTC index", line_number)?;
    Ok(Some(ScoCtc {
        var_name: var,
        texture_dir: dir,
        default_index: idx,
    }))
}

/// Applies the block if its keyword is one of this module's; says whether it was.
pub(super) fn apply(obj: &mut SceneryObject, block: &OmsiBlock) -> Result<bool, SceneryParseError> {
    match block.keyword.as_str() {
        "texttexture" | "texttexture_enh" => {
            if let Some(tex) = parse_text_texture(&block.parameters, block.line_number)? {
                obj.text_textures.push(tex);
            }
        }
        "ctc" => {
            if let Some(ctc) = parse_ctc(&block.parameters, block.line_number)? {
                obj.ctc = Some(ctc);
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
