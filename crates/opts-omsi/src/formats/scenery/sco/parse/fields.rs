//! Number parsers of `.sco` fields: each names its field and line in the error.

use super::super::*;
pub(in crate::formats::scenery::sco) fn parse_f32(
    s: &str,
    field: &'static str,
    line: usize,
) -> Result<f32, SceneryParseError> {
    parse_float(s).map_err(|_| SceneryParseError::InvalidNumber {
        field,
        value: s.to_string(),
        line,
    })
}

pub(in crate::formats::scenery::sco) fn parse_u32(
    s: &str,
    field: &'static str,
    line: usize,
) -> Result<u32, SceneryParseError> {
    s.trim()
        .parse::<u32>()
        .map_err(|_| SceneryParseError::InvalidNumber {
            field,
            value: s.to_string(),
            line,
        })
}

pub(in crate::formats::scenery::sco) fn parse_u8(s: &str, field: &'static str, line: usize) -> Result<u8, SceneryParseError> {
    s.trim()
        .parse::<u8>()
        .map_err(|_| SceneryParseError::InvalidNumber {
            field,
            value: s.to_string(),
            line,
        })
}

pub(in crate::formats::scenery::sco) fn parse_usize(
    s: &str,
    field: &'static str,
    line: usize,
) -> Result<usize, SceneryParseError> {
    s.trim()
        .parse::<usize>()
        .map_err(|_| SceneryParseError::InvalidNumber {
            field,
            value: s.to_string(),
            line,
        })
}
