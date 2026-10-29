//! Parser implementation for vehicle rendering manifests (`model.cfg`).

use glam::Vec3;

use crate::formats::scenery::light::{parse_light_enh, parse_light_enh_2};
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

struct ModelParserState {
    config: ModelConfig,
    current_lod_idx: Option<usize>,
    current_mesh_idx: Option<usize>,
}

impl ModelParserState {
    fn new() -> Self {
        Self {
            config: ModelConfig::new(),
            current_lod_idx: None,
            current_mesh_idx: None,
        }
    }

    fn active_mesh_mut(&mut self) -> Option<&mut ModelMesh> {
        let lod_idx = self.current_lod_idx?;
        let mesh_idx = self.current_mesh_idx?;
        self.config.lods.get_mut(lod_idx)?.meshes.get_mut(mesh_idx)
    }

    fn handle_lod(&mut self, block: &OmsiBlock) -> Result<(), ModelParseError> {
        let params = require_params(block, 1)?;
        let threshold = parse_number::<f32>(&params[0], block.line_number)?;
        self.config.lods.push(ModelLod {
            threshold,
            meshes: Vec::new(),
        });
        self.current_lod_idx = Some(self.config.lods.len().saturating_sub(1));
        self.current_mesh_idx = None;
        Ok(())
    }

    fn handle_mesh(&mut self, block: &OmsiBlock) -> Result<(), ModelParseError> {
        let params = require_params(block, 1)?;
        let mesh = ModelMesh::new(params[0].clone());
        let lod_idx = match self.current_lod_idx {
            Some(idx) => idx,
            None => {
                self.config.lods.push(ModelLod {
                    threshold: 0.0,
                    meshes: Vec::new(),
                });
                let idx = self.config.lods.len().saturating_sub(1);
                self.current_lod_idx = Some(idx);
                idx
            }
        };
        if let Some(lod) = self.config.lods.get_mut(lod_idx) {
            lod.meshes.push(mesh);
            self.current_mesh_idx = Some(lod.meshes.len().saturating_sub(1));
        }
        Ok(())
    }

    fn handle_light_enh(&mut self, block: &OmsiBlock) {
        let parsed = if block.keyword == "light_enh" {
            parse_light_enh(block)
        } else {
            parse_light_enh_2(block)
        };
        match parsed {
            Ok(mut light) => {
                light.lod_threshold = self
                    .current_lod_idx
                    .and_then(|i| self.config.lods.get(i))
                    .map(|l| l.threshold);
                light.after_mesh = self.current_mesh_idx;
                self.config.lights.push(light);
            }
            Err(e) => self.config.warnings.push(e),
        }
    }

    fn handle_mesh_property_block(&mut self, block: &OmsiBlock) -> Result<(), ModelParseError> {
        if let Some(mesh) = self.active_mesh_mut() {
            handle_mesh_property(mesh, block)?;
        } else {
            self.config
                .unknown_fields
                .entry(block.keyword.clone())
                .or_default()
                .push(block.parameters.clone());
        }
        Ok(())
    }

    fn handle_material_modifier_block(&mut self, block: &OmsiBlock) -> Result<(), ModelParseError> {
        if let Some(mesh) = self.active_mesh_mut() {
            handle_material_block(mesh, block)?;
        } else {
            self.config
                .unknown_fields
                .entry(block.keyword.clone())
                .or_default()
                .push(block.parameters.clone());
        }
        Ok(())
    }

    fn handle_block(&mut self, block: &OmsiBlock) -> Result<(), ModelParseError> {
        match block.keyword.as_str() {
            "lod" => self.handle_lod(block)?,
            "mesh" => self.handle_mesh(block)?,
            "viewpoint"
            | "illumination_interior"
            | "visible"
            | "mouseevent"
            | "animparent"
            | "mesh_ident"
            | "smoothskin"
            | "setbone"
            | "isshadow" => self.handle_mesh_property_block(block)?,
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
            | "usescripttexture"
            | "alphascale"
            | "texcoordtransy" => self.handle_material_modifier_block(block)?,
            "newanim" => {
                let anim = parse_newanim(block)?;
                if let Some(mesh) = self.active_mesh_mut() {
                    mesh.anims.push(anim);
                }
            }
            "interiorlight" => {
                let light = parse_interior_light(block)?;
                if let Some(mesh) = self.active_mesh_mut() {
                    mesh.interior_lights.push(light);
                } else {
                    self.config.interior_lights.push(light);
                }
            }
            "spotlight" => {
                let light = parse_spotlight(block)?;
                if let Some(mesh) = self.active_mesh_mut() {
                    mesh.spotlights.push(light);
                } else {
                    self.config.spotlights.push(light);
                }
            }
            "light_enh" | "light_enh_2" => self.handle_light_enh(block),
            "tex_detail_factor" | "ctc" | "ctctexture" | "texttexture" | "texttexture_enh"
            | "scripttexture" => {
                handle_global_directive(&mut self.config, block)?;
            }
            _ => {
                if let Some(mesh) = self.active_mesh_mut() {
                    mesh.unknown_fields
                        .entry(block.keyword.clone())
                        .or_default()
                        .push(block.parameters.clone());
                } else {
                    self.config
                        .unknown_fields
                        .entry(block.keyword.clone())
                        .or_default()
                        .push(block.parameters.clone());
                }
            }
        }
        Ok(())
    }
}

/// Parses an OMSI `model.cfg` file into a structured [`ModelConfig`].
pub fn parse_model(text: &str) -> Result<ModelConfig, ModelParseError> {
    let blocks = scan_blocks_with(text, model_arity);
    let mut state = ModelParserState::new();
    for block in &blocks {
        state.handle_block(block)?;
    }
    Ok(state.config)
}

mod global;
mod mesh;

use global::handle_global_directive;
use mesh::{handle_material_block, handle_mesh_property, parse_interior_light, parse_spotlight};
