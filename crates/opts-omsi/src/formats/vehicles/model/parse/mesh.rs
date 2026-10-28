//! Mesh properties, material modifiers, interior light, and spotlight block parsers for `model.cfg`.

use super::*;

pub(super) fn handle_mesh_property(
    mesh: &mut ModelMesh,
    block: &OmsiBlock,
) -> Result<(), ModelParseError> {
    match block.keyword.as_str() {
        "viewpoint" => {
            let params = require_params(block, 1)?;
            let mask = parse_number::<u32>(&params[0], block.line_number)?;
            mesh.viewpoint = ViewpointFlags::new(mask);
        }
        "illumination_interior" => {
            let params = require_params(block, 4)?;
            let a = parse_number::<i32>(&params[0], block.line_number)?;
            let b = parse_number::<i32>(&params[1], block.line_number)?;
            let c = parse_number::<i32>(&params[2], block.line_number)?;
            let d = parse_number::<i32>(&params[3], block.line_number)?;
            mesh.illumination_interior = Some([a, b, c, d]);
        }
        "visible" => {
            let params = require_params(block, 2)?;
            let expected_value = parse_number::<f32>(&params[1], block.line_number)?;
            mesh.visibility.push(MeshVisibility {
                variable: params[0].clone(),
                expected_value,
            });
        }
        "mouseevent" => {
            let params = require_params(block, 1)?;
            mesh.mouseevent = Some(params[0].clone());
        }
        "animparent" => {
            let params = require_params(block, 1)?;
            mesh.animparent = Some(params[0].clone());
        }
        "mesh_ident" => {
            let params = require_params(block, 1)?;
            mesh.mesh_ident = Some(params[0].clone());
        }
        "smoothskin" => {
            mesh.smoothskin = true;
        }
        "setbone" => {
            let params = require_params(block, 2)?;
            // Human skeleton slots are negative (-2 … -14); vehicle bones are non-negative.
            let index = parse_number::<i32>(&params[1], block.line_number)?;
            mesh.bones.push(MeshBone {
                name: params[0].clone(),
                index,
            });
        }
        "isshadow" => {
            mesh.isshadow = true;
        }
        _ => {}
    }
    Ok(())
}

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

fn apply_material_modifier(
    mat: &mut ModelMaterial,
    block: &OmsiBlock,
) -> Result<(), ModelParseError> {
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
        "matl_nightmap" => {
            if let Some(texture) = block.parameters.first() {
                mat.night_map = Some(texture.clone());
            }
        }
        "matl_lightmap" => {
            // The variable is optional in the wild (e.g. Citybus 530); without one the lightmap
            // has no switch. If parameters are empty, ignore gracefully.
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

pub(super) fn parse_interior_light(block: &OmsiBlock) -> Result<InteriorLight, ModelParseError> {
    let params = require_params(block, 8)?;
    let range = parse_number::<f32>(&params[1], block.line_number)?;
    let r = parse_number::<u8>(&params[2], block.line_number)?;
    let g = parse_number::<u8>(&params[3], block.line_number)?;
    let b = parse_number::<u8>(&params[4], block.line_number)?;
    let x = parse_number::<f32>(&params[5], block.line_number)?;
    let y = parse_number::<f32>(&params[6], block.line_number)?;
    let z = parse_number::<f32>(&params[7], block.line_number)?;

    Ok(InteriorLight {
        variable: params[0].clone(),
        range,
        color_rgb: [r, g, b],
        pos: Vec3::new(x, y, z),
    })
}

pub(super) fn parse_spotlight(block: &OmsiBlock) -> Result<Spotlight, ModelParseError> {
    let params = require_params(block, 12)?;
    let px = parse_number::<f32>(&params[0], block.line_number)?;
    let py = parse_number::<f32>(&params[1], block.line_number)?;
    let pz = parse_number::<f32>(&params[2], block.line_number)?;
    let dx = parse_number::<f32>(&params[3], block.line_number)?;
    let dy = parse_number::<f32>(&params[4], block.line_number)?;
    let dz = parse_number::<f32>(&params[5], block.line_number)?;
    let r = parse_number::<u8>(&params[6], block.line_number)?;
    let g = parse_number::<u8>(&params[7], block.line_number)?;
    let b = parse_number::<u8>(&params[8], block.line_number)?;
    let range = parse_number::<f32>(&params[9], block.line_number)?;
    let inner_cone = parse_number::<f32>(&params[10], block.line_number)?;
    let outer_cone = parse_number::<f32>(&params[11], block.line_number)?;

    Ok(Spotlight {
        pos: Vec3::new(px, py, pz),
        dir: Vec3::new(dx, dy, dz),
        color_rgb: [r, g, b],
        range,
        inner_cone,
        outer_cone,
    })
}
