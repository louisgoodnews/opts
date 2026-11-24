//! Mesh property blocks of `model.cfg` (`viewpoint`, `visible`, `setbone`, and the other mesh flags).

use super::{parse_number, require_params};
use crate::formats::text::OmsiBlock;
use crate::formats::vehicles::model::ast::{MeshBone, MeshVisibility, ModelMesh, ViewpointFlags};
use crate::formats::vehicles::model::error::ModelParseError;

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
