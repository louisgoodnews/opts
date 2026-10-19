//! Parser for vehicle mesh animation blocks (`[newanim]`).

use glam::Vec3;

use crate::formats::text::OmsiBlock;
use crate::formats::vehicles::model::ast::{AnimAction, ModelAnim};
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

/// Parses an OMSI `[newanim]` keyword block into a [`ModelAnim`].
pub fn parse_newanim(block: &OmsiBlock) -> Result<ModelAnim, ModelParseError> {
    let mut anim = ModelAnim::default();
    let mut i = 0;
    let params = &block.parameters;

    while i < params.len() {
        let tag = params[i].trim().to_ascii_lowercase();
        if handle_origin_subtag(&mut anim, &tag, params, &mut i, block.line_number)? {
            continue;
        }
        if handle_action_subtag(&mut anim, &tag, params, &mut i, block.line_number)? {
            continue;
        }
        if handle_tuning_subtag(&mut anim, &tag, params, &mut i, block.line_number)? {
            continue;
        }

        anim.unknown_properties
            .push((params[i].clone(), Vec::new()));
        i += 1;
    }

    Ok(anim)
}

fn handle_origin_subtag(
    anim: &mut ModelAnim,
    tag: &str,
    params: &[String],
    i: &mut usize,
    line: usize,
) -> Result<bool, ModelParseError> {
    match tag {
        "origin_trans" => {
            if *i + 3 < params.len() {
                let x = parse_number::<f32>(&params[*i + 1], line)?;
                let y = parse_number::<f32>(&params[*i + 2], line)?;
                let z = parse_number::<f32>(&params[*i + 3], line)?;
                anim.origin_trans = Some(Vec3::new(x, y, z));
                *i += 4;
                Ok(true)
            } else {
                Err(ModelParseError::MissingParameters {
                    keyword: "origin_trans".to_string(),
                    line,
                    expected: 3,
                    found: params.len().saturating_sub(*i + 1),
                })
            }
        }
        "origin_rot_x" => {
            anim.origin_rot.x = parse_single_param("origin_rot_x", params, i, line)?;
            Ok(true)
        }
        "origin_rot_y" => {
            anim.origin_rot.y = parse_single_param("origin_rot_y", params, i, line)?;
            Ok(true)
        }
        "origin_rot_z" => {
            anim.origin_rot.z = parse_single_param("origin_rot_z", params, i, line)?;
            Ok(true)
        }
        "origin_from_mesh" => {
            anim.origin_from_mesh = true;
            *i += 1;
            Ok(true)
        }
        _ => Ok(false),
    }
}

fn handle_action_subtag(
    anim: &mut ModelAnim,
    tag: &str,
    params: &[String],
    i: &mut usize,
    line: usize,
) -> Result<bool, ModelParseError> {
    match tag {
        "anim_rot" => {
            if *i + 2 < params.len() {
                let factor = parse_number::<f32>(&params[*i + 2], line)?;
                anim.anim_rot = Some(AnimAction {
                    variable: params[*i + 1].clone(),
                    factor,
                });
                *i += 3;
                Ok(true)
            } else {
                Err(ModelParseError::MissingParameters {
                    keyword: "anim_rot".to_string(),
                    line,
                    expected: 2,
                    found: params.len().saturating_sub(*i + 1),
                })
            }
        }
        "anim_trans" => {
            if *i + 2 < params.len() {
                let factor = parse_number::<f32>(&params[*i + 2], line)?;
                anim.anim_trans = Some(AnimAction {
                    variable: params[*i + 1].clone(),
                    factor,
                });
                *i += 3;
                Ok(true)
            } else {
                Err(ModelParseError::MissingParameters {
                    keyword: "anim_trans".to_string(),
                    line,
                    expected: 2,
                    found: params.len().saturating_sub(*i + 1),
                })
            }
        }
        "anim_val" => {
            if *i + 1 < params.len() {
                anim.anim_val = Some(params[*i + 1].clone());
                *i += 2;
                Ok(true)
            } else {
                Err(ModelParseError::MissingParameters {
                    keyword: "anim_val".to_string(),
                    line,
                    expected: 1,
                    found: 0,
                })
            }
        }
        "anim_maxval" => {
            anim.anim_maxval = Some(parse_single_param("anim_maxval", params, i, line)?);
            Ok(true)
        }
        _ => Ok(false),
    }
}

fn handle_tuning_subtag(
    anim: &mut ModelAnim,
    tag: &str,
    params: &[String],
    i: &mut usize,
    line: usize,
) -> Result<bool, ModelParseError> {
    match tag {
        "delay" => {
            anim.delay = Some(parse_single_param("delay", params, i, line)?);
            Ok(true)
        }
        "maxspeed" => {
            anim.maxspeed = Some(parse_single_param("maxspeed", params, i, line)?);
            Ok(true)
        }
        "offset" => {
            anim.offset = Some(parse_single_param("offset", params, i, line)?);
            Ok(true)
        }
        _ => Ok(false),
    }
}

fn parse_single_param(
    keyword: &str,
    params: &[String],
    i: &mut usize,
    line: usize,
) -> Result<f32, ModelParseError> {
    if *i + 1 < params.len() {
        let val = parse_number::<f32>(&params[*i + 1], line)?;
        *i += 2;
        Ok(val)
    } else {
        Err(ModelParseError::MissingParameters {
            keyword: keyword.to_string(),
            line,
            expected: 1,
            found: 0,
        })
    }
}
