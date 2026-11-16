//! The action sub-keywords of `[newanim]`: `anim_rot`, `anim_trans`, `anim_val`, `anim_maxval`.

use super::{parse_number, parse_single_param};
use crate::formats::vehicles::model::ast::{AnimAction, ModelAnim};
use crate::formats::vehicles::model::error::ModelParseError;

pub(super) fn handle_action_subtag(
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
