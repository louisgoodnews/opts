//! Parser state of `model.cfg`: the current LOD and mesh, and the handlers that open them or
//! attach a property block to them.

use crate::formats::scenery::light::{parse_light_enh, parse_light_enh_2};
use crate::formats::text::OmsiBlock;
use crate::formats::vehicles::model::ast::{ModelConfig, ModelLod, ModelMesh};
use crate::formats::vehicles::model::error::ModelParseError;

use super::material::handle_material_block;
use super::mesh::handle_mesh_property;
use super::{parse_number, require_params};

pub(super) struct ModelParserState {
    pub(super) config: ModelConfig,
    current_lod_idx: Option<usize>,
    current_mesh_idx: Option<usize>,
}

impl ModelParserState {
    pub(super) fn new() -> Self {
        Self {
            config: ModelConfig::new(),
            current_lod_idx: None,
            current_mesh_idx: None,
        }
    }

    pub(super) fn active_mesh_mut(&mut self) -> Option<&mut ModelMesh> {
        let lod_idx = self.current_lod_idx?;
        let mesh_idx = self.current_mesh_idx?;
        self.config.lods.get_mut(lod_idx)?.meshes.get_mut(mesh_idx)
    }

    pub(super) fn handle_lod(&mut self, block: &OmsiBlock) -> Result<(), ModelParseError> {
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

    pub(super) fn handle_mesh(&mut self, block: &OmsiBlock) -> Result<(), ModelParseError> {
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

    pub(super) fn handle_light_enh(&mut self, block: &OmsiBlock) {
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

    pub(super) fn handle_mesh_property_block(
        &mut self,
        block: &OmsiBlock,
    ) -> Result<(), ModelParseError> {
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

    pub(super) fn handle_material_modifier_block(
        &mut self,
        block: &OmsiBlock,
    ) -> Result<(), ModelParseError> {
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
}
