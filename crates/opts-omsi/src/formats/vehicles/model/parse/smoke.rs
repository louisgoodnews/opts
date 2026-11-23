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

#[cfg(test)]
mod tests {
    use super::*;

    fn block(params: &[&str]) -> OmsiBlock {
        OmsiBlock {
            keyword: "smoke".to_string(),
            parameters: params.iter().map(|s| s.to_string()).collect(),
            line_number: 1,
        }
    }

    #[test]
    fn reads_position_direction_and_variables() {
        let e = parse_smoke(&block(&[
            "0",
            "-5.5",
            "1.2",
            "0",
            "1",
            "0",
            "motor_on",
            "1",
            "exhaust_freq",
        ]))
        .expect("smoke block parses");
        assert_eq!(e.pos, Vec3::new(0.0, -5.5, 1.2));
        assert_eq!(e.dir, Vec3::new(0.0, 1.0, 0.0));
        assert_eq!(e.variable.as_deref(), Some("motor_on"));
        assert_eq!(e.freq_variable.as_deref(), Some("exhaust_freq"));
    }

    #[test]
    fn optional_variables_may_be_absent() {
        let e = parse_smoke(&block(&["1", "2", "3", "0", "0", "1"])).expect("parses");
        assert_eq!(e.variable, None);
        assert_eq!(e.freq_variable, None);
    }

    #[test]
    fn short_block_is_an_error() {
        assert!(parse_smoke(&block(&["1", "2"])).is_err());
    }
}
