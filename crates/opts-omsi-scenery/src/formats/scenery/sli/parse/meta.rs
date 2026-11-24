//! Metadata and texture blocks of a spline profile.

use super::*;

pub(super) fn handle_meta(
    block: &OmsiBlock,
    sli: &mut SplineProfile,
) -> Result<(), SceneryParseError> {
    match block.keyword.as_str() {
        "length" => {
            if let Some(val) = block.parameters.first() {
                sli.length = Some(parse_f32(val, "length", block.line_number)?);
            }
        }
        "halfcantwidth" => {
            if let Some(val) = block.parameters.first() {
                sli.half_cant_width = Some(parse_f32(val, "halfcantwidth", block.line_number)?);
            }
        }
        "scaletexbylength" => sli.scale_tex_by_length = true,
        "rendertype" => {
            if let Some(val) = block.parameters.first() {
                sli.render_type = Some(val.clone());
            }
        }
        "onlyeditor" => sli.only_editor = true,
        "heightprofile" | "heightproflile" | "heigthprofile" if block.parameters.len() >= 4 => {
            let xs = parse_f32(
                &block.parameters[0],
                "heightprofile x_start",
                block.line_number,
            )?;
            let xe = parse_f32(
                &block.parameters[1],
                "heightprofile x_end",
                block.line_number,
            )?;
            let zs = parse_f32(
                &block.parameters[2],
                "heightprofile z_start",
                block.line_number,
            )?;
            let ze = parse_f32(
                &block.parameters[3],
                "heightprofile z_end",
                block.line_number,
            )?;
            sli.height_profiles.push(SplineHeightProfile {
                x_start: xs,
                x_end: xe,
                z_start: zs,
                z_end: ze,
            });
        }
        _ => {}
    }
    Ok(())
}

pub(super) fn handle_texture(
    block: &OmsiBlock,
    sli: &mut SplineProfile,
) -> Result<(), SceneryParseError> {
    match block.keyword.as_str() {
        "texture" => {
            if let Some(file) = block.parameters.first() {
                sli.textures.push(SplineTexture {
                    file: file.clone(),
                    material_index: None,
                    alpha_mode: None,
                });
            }
        }
        "matl" if block.parameters.len() >= 2 => {
            let idx = parse_usize(&block.parameters[1], "matl index", block.line_number)?;
            if let Some(tex) = sli.textures.last_mut() {
                tex.material_index = Some(idx);
            }
        }
        "matl_alpha" => {
            if let Some(val) = block.parameters.first() {
                let mode = parse_u32(val, "matl_alpha", block.line_number)?;
                if let Some(tex) = sli.textures.last_mut() {
                    tex.alpha_mode = Some(mode);
                }
            }
        }
        _ => {}
    }
    Ok(())
}
