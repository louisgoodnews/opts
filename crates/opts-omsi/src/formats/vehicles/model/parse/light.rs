//! Interior light and spotlight blocks of `model.cfg`.

use glam::Vec3;

use super::{parse_number, require_params};
use crate::formats::text::OmsiBlock;
use crate::formats::vehicles::model::ast::{InteriorLight, Spotlight};
use crate::formats::vehicles::model::error::ModelParseError;

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
