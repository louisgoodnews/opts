//! Block dispatch of `model.cfg`: routes each keyword block to its handler.

use crate::formats::text::OmsiBlock;
use crate::formats::vehicles::model::anim::parse_newanim;
use crate::formats::vehicles::model::error::ModelParseError;

use super::global::handle_global_directive;
use super::light::{parse_interior_light, parse_spotlight};
use super::state::ModelParserState;

impl ModelParserState {
    pub(super) fn handle_block(&mut self, block: &OmsiBlock) -> Result<(), ModelParseError> {
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
