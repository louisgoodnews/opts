//! Material modifier blocks of `model.cfg` (`matl_*`, texture-change and alpha keywords).

use super::{parse_number, require_params};
use crate::formats::text::OmsiBlock;
use crate::formats::vehicles::model::ast::{
    BumpMap, EnvMap, FreeTex, LightMap, ModelMaterial, ModelMesh, TextureChange,
};
use crate::formats::vehicles::model::error::ModelParseError;

pub(super) fn handle_material_block(
    mesh: &mut ModelMesh,
    block: &OmsiBlock,
) -> Result<(), ModelParseError> {
    if block.keyword == "matl" {
        let params = require_params(block, 2)?;
        let slot_index = parse_number::<u32>(&params[1], block.line_number)?;
        mesh.materials.push(ModelMaterial {
            texture: params[0].clone(),
            slot_index,
            ..Default::default()
        });
        return Ok(());
    }

    if mesh.materials.is_empty() {
        mesh.materials.push(ModelMaterial::default());
    }
    let mat_idx = mesh.materials.len().saturating_sub(1);
    if let Some(mat) = mesh.materials.get_mut(mat_idx) {
        apply_material_modifier(mat, block)?;
    }
    Ok(())
}

fn apply_map_modifier(mat: &mut ModelMaterial, block: &OmsiBlock) -> Result<bool, ModelParseError> {
    match block.keyword.as_str() {
        "matl_nightmap" => {
            if let Some(texture) = block.parameters.first() {
                mat.night_map = Some(texture.clone());
            }
        }
        "matl_lightmap" => {
            if let Some(texture) = block.parameters.first() {
                mat.light_map = Some(LightMap {
                    texture: texture.clone(),
                    variable: block.parameters.get(1).cloned().unwrap_or_default(),
                });
            }
        }
        "matl_envmap" => {
            let params = require_params(block, 2)?;
            let factor = parse_number::<f32>(&params[1], block.line_number)?;
            mat.env_map = Some(EnvMap {
                texture: params[0].clone(),
                factor,
            });
        }
        "matl_envmap_mask" => {
            let params = require_params(block, 1)?;
            mat.env_map_mask = Some(params[0].clone());
        }
        "matl_bumpmap" => {
            let params = require_params(block, 2)?;
            let factor = parse_number::<f32>(&params[1], block.line_number)?;
            mat.bump_map = Some(BumpMap {
                texture: params[0].clone(),
                factor,
            });
        }
        "matl_transmap" => {
            let trans = block
                .parameters
                .first()
                .filter(|p| !p.trim().is_empty())
                .cloned();
            if let Some(t) = &trans {
                let trimmed = t.trim();
                if trimmed.starts_with(r"\S:") || trimmed.starts_with(r"\s:") {
                    if let Ok(idx) = trimmed[3..].trim().parse::<usize>() {
                        mat.script_texture_index = Some(idx);
                    }
                }
            }
            mat.trans_map = trans;
        }
        "matl_freetex" => {
            let params = require_params(block, 2)?;
            mat.free_tex = Some(FreeTex {
                texture: params[0].clone(),
                variable: params[1].clone(),
            });
        }
        _ => return Ok(false),
    }
    Ok(true)
}

fn apply_material_modifier(
    mat: &mut ModelMaterial,
    block: &OmsiBlock,
) -> Result<(), ModelParseError> {
    if apply_map_modifier(mat, block)? {
        return Ok(());
    }
    match block.keyword.as_str() {
        "matl_alpha" => {
            let params = require_params(block, 1)?;
            mat.alpha_mode = Some(parse_number::<u32>(&params[0], block.line_number)?);
        }
        "matl_change" => {
            let params = require_params(block, 2)?;
            let idx = parse_number::<u32>(&params[1], block.line_number)?;
            mat.texture_changes.push(TextureChange {
                texture: params[0].clone(),
                index: idx,
                variable: params.get(2).cloned().unwrap_or_default(),
            });
        }
        "matl_nozwrite" => mat.no_z_write = true,
        "matl_nozcheck" => mat.no_z_check = true,
        "matl_texadress_clamp" => mat.clamp_addressing = true,
        "usetexttexture" => {
            let params = require_params(block, 1)?;
            mat.text_texture_index = Some(parse_number::<usize>(&params[0], block.line_number)?);
        }
        "usescripttexture" => {
            let params = require_params(block, 1)?;
            mat.script_texture_index = Some(parse_number::<usize>(&params[0], block.line_number)?);
        }
        "alphascale" => {
            let params = require_params(block, 1)?;
            mat.alpha_scale_var = Some(params[0].clone());
        }
        "texcoordtransy" => {
            let params = require_params(block, 1)?;
            mat.tex_coord_trans_y_var = Some(params[0].clone());
        }
        _ => {
            mat.unknown_modifiers
                .push((block.keyword.clone(), block.parameters.clone()));
        }
    }
    Ok(())
}
