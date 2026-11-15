//! Parser implementation for vehicle rendering manifests (`model.cfg`).

use crate::formats::text::{scan_blocks_with, OmsiBlock};
use crate::formats::vehicles::model::ast::ModelConfig;
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
    let mut state = ModelParserState::new();
    for block in &blocks {
        state.handle_block(block)?;
    }
    Ok(state.config)
}

mod dispatch;
mod global;
mod light;
mod material;
mod mesh;
mod state;

use state::ModelParserState;
