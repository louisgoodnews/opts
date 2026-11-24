//! The `[smoke]` block of `model.cfg`.

use glam::Vec3;

use super::{parse_number, require_params};
use crate::formats::text::OmsiBlock;
use crate::formats::vehicles::model::ast::SmokeEmitter;
use crate::formats::vehicles::model::error::ModelParseError;

/// Reads a `[smoke]` block: at least position and direction, then optional variable names.
pub(super) fn parse_smoke(block: &OmsiBlock) -> Result<SmokeEmitter, ModelParseError> {
    let p = require_params(block, 6)?;
    let n = |i: usize| parse_number::<f32>(&p[i], block.line_number);
    let name = |i: usize| {
        p.get(i)
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    };
    Ok(SmokeEmitter {
        pos: Vec3::new(n(0)?, n(1)?, n(2)?),
        dir: Vec3::new(n(3)?, n(4)?, n(5)?),
        variable: name(6),
        freq_variable: name(8),
    })
}
