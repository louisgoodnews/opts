//! Parser implementation for vehicle rendering manifests (`model.cfg`).

use glam::Vec3;

use crate::formats::text::{scan_blocks_with, OmsiBlock};
use crate::formats::vehicles::model::anim::parse_newanim;
use crate::formats::vehicles::model::ast::{
    BumpMap, CtcScheme, CtcTexture, EnvMap, FreeTex, InteriorLight, LightMap, MeshBone,
    MeshVisibility, ModelConfig, ModelLod, ModelMaterial, ModelMesh, ScriptTexture, Spotlight,
    TextTexture, TextureChange, ViewpointFlags,
};
use crate::formats::vehicles::model::error::ModelParseError;

fn parse_number<T: std::str::FromStr>(s: &str, line: usize) -> Result<T, ModelParseError> {
    let normalized = s.trim().replace(',', ".");
    normalized
        .parse::<T>()
        .map_err(|_| ModelParseError::InvalidNumber {
            line,
            value: s.trim().to_string(),
        })
}

fn require_params(block: &OmsiBlock, expected: usize) -> Result<&[String], ModelParseError> {
    if block.parameters.len() < expected {
        Err(ModelParseError::MissingParameters {
            keyword: block.keyword.clone(),
            line: block.line_number,
            expected,
            found: block.parameters.len(),
        })
    } else {
        Ok(&block.parameters)
    }
}

/// Keywords of `model.cfg` that take a fixed number of parameter lines (`docs/src/formats/text/vehicles/model_cfg.md`):
/// zero for the flags, one for `[matl_transmap]` whose blank line is an empty value. Only these are read
/// literally; text after them up to the next keyword is not a parameter. Everything else keeps the
/// free-form reading (some keywords have optional parameters).
fn model_arity(keyword: &str) -> Option<usize> {
    match keyword {
        "matl_transmap" => Some(1),
        "matl_item"
        | "matl_nozwrite"
        | "matl_nozcheck"
        | "matl_nowrite"
        | "matl_texadress_clamp"
        | "isshadow"
        | "shadow"
        | "smoothskin"
        | "nocollision"
        | "nodistancecheck" => Some(0),
        _ => None,
    }
}

/// Parses an OMSI `model.cfg` file into a structured [`ModelConfig`].
pub fn parse_model(text: &str) -> Result<ModelConfig, ModelParseError> {
    let blocks = scan_blocks_with(text, model_arity);
    let mut config = ModelConfig::new();
    let mut current_lod_idx: Option<usize> = None;
    let mut current_mesh_idx: Option<usize> = None;

    for block in &blocks {
        match block.keyword.as_str() {
            "lod" => {
                let params = require_params(block, 1)?;
                let threshold = parse_number::<f32>(&params[0], block.line_number)?;
                config.lods.push(ModelLod {
                    threshold,
                    meshes: Vec::new(),
                });
                current_lod_idx = Some(config.lods.len().saturating_sub(1));
                current_mesh_idx = None;
            }
            "mesh" => {
                let params = require_params(block, 1)?;
                let mesh = ModelMesh::new(params[0].clone());
                let lod_idx = match current_lod_idx {
                    Some(idx) => idx,
                    None => {
                        config.lods.push(ModelLod {
                            threshold: 0.0,
                            meshes: Vec::new(),
                        });
                        let idx = config.lods.len().saturating_sub(1);
                        current_lod_idx = Some(idx);
                        idx
                    }
                };
                if let Some(lod) = config.lods.get_mut(lod_idx) {
                    lod.meshes.push(mesh);
                    current_mesh_idx = Some(lod.meshes.len().saturating_sub(1));
                }
            }
            "viewpoint"
            | "illumination_interior"
            | "visible"
            | "mouseevent"
            | "animparent"
            | "mesh_ident"
            | "smoothskin"
            | "setbone"
            | "isshadow" => {
                if let Some(mesh) =
                    get_active_mesh_mut(&mut config, current_lod_idx, current_mesh_idx)
                {
                    handle_mesh_property(mesh, block)?;
                } else {
                    config
                        .unknown_fields
                        .entry(block.keyword.clone())
                        .or_default()
                        .push(block.parameters.clone());
                }
            }
            "matl"
            | "matl_alpha"
            | "matl_change"
            | "matl_item"
            | "matl_nightmap"
            | "matl_lightmap"
            | "matl_envmap"
            | "matl_envmap_mask"
            | "matl_bumpmap"
            | "matl_transmap"
            | "matl_freetex"
            | "matl_nozwrite"
            | "matl_nozcheck"
            | "matl_texadress_clamp"
            | "usetexttexture"
            | "alphascale"
            | "texcoordtransy" => {
                if let Some(mesh) =
                    get_active_mesh_mut(&mut config, current_lod_idx, current_mesh_idx)
                {
                    handle_material_block(mesh, block)?;
                } else {
                    config
                        .unknown_fields
                        .entry(block.keyword.clone())
                        .or_default()
                        .push(block.parameters.clone());
                }
            }
            "newanim" => {
                let anim = parse_newanim(block)?;
                if let Some(mesh) =
                    get_active_mesh_mut(&mut config, current_lod_idx, current_mesh_idx)
                {
                    mesh.anims.push(anim);
                }
            }
            "interiorlight" => {
                let light = parse_interior_light(block)?;
                if let Some(mesh) =
                    get_active_mesh_mut(&mut config, current_lod_idx, current_mesh_idx)
                {
                    mesh.interior_lights.push(light);
                } else {
                    config.interior_lights.push(light);
                }
            }
            "spotlight" => {
                let light = parse_spotlight(block)?;
                if let Some(mesh) =
                    get_active_mesh_mut(&mut config, current_lod_idx, current_mesh_idx)
                {
                    mesh.spotlights.push(light);
                } else {
                    config.spotlights.push(light);
                }
            }
            "tex_detail_factor" | "ctc" | "ctctexture" | "texttexture" | "texttexture_enh"
            | "scripttexture" => {
                handle_global_directive(&mut config, block)?;
            }
            _ => {
                if let Some(mesh) =
                    get_active_mesh_mut(&mut config, current_lod_idx, current_mesh_idx)
                {
                    mesh.unknown_fields
                        .entry(block.keyword.clone())
                        .or_default()
                        .push(block.parameters.clone());
                } else {
                    config
                        .unknown_fields
                        .entry(block.keyword.clone())
                        .or_default()
                        .push(block.parameters.clone());
                }
            }
        }
    }

    Ok(config)
}

fn get_active_mesh_mut(
    config: &mut ModelConfig,
    current_lod_idx: Option<usize>,
    current_mesh_idx: Option<usize>,
) -> Option<&mut ModelMesh> {
    let lod_idx = current_lod_idx?;
    let mesh_idx = current_mesh_idx?;
    config.lods.get_mut(lod_idx)?.meshes.get_mut(mesh_idx)
}

fn handle_mesh_property(mesh: &mut ModelMesh, block: &OmsiBlock) -> Result<(), ModelParseError> {
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

fn handle_material_block(mesh: &mut ModelMesh, block: &OmsiBlock) -> Result<(), ModelParseError> {
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
            mat.trans_map = block
                .parameters
                .first()
                .filter(|p| !p.trim().is_empty())
                .cloned();
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

fn parse_interior_light(block: &OmsiBlock) -> Result<InteriorLight, ModelParseError> {
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

fn parse_spotlight(block: &OmsiBlock) -> Result<Spotlight, ModelParseError> {
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

fn handle_global_directive(
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
