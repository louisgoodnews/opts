//! Scalar fields of a `global.cfg` block and the raw lines of a block.

use super::*;

pub(super) fn parse_f32(s: &str, field: &'static str, line: usize) -> Result<f32, MapParseError> {
    parse_float(s).map_err(|_| MapParseError::InvalidNumber {
        field,
        value: s.to_string(),
        line,
    })
}

pub(super) fn parse_i32(s: &str, field: &'static str, line: usize) -> Result<i32, MapParseError> {
    s.trim()
        .parse::<i32>()
        .map_err(|_| MapParseError::InvalidNumber {
            field,
            value: s.to_string(),
            line,
        })
}

pub(super) fn parse_u32(s: &str, field: &'static str, line: usize) -> Result<u32, MapParseError> {
    s.trim()
        .parse::<u32>()
        .map_err(|_| MapParseError::InvalidNumber {
            field,
            value: s.to_string(),
            line,
        })
}

pub(super) fn parse_usize(
    s: &str,
    field: &'static str,
    line: usize,
) -> Result<usize, MapParseError> {
    s.trim()
        .parse::<usize>()
        .map_err(|_| MapParseError::InvalidNumber {
            field,
            value: s.to_string(),
            line,
        })
}

/// The lines after the keyword line `keyword_line` (1-based) up to the next keyword line, trimmed
/// but otherwise untouched: blank lines and lines starting with `#` or `//` are kept.
pub(super) fn raw_block_lines(text: &str, keyword_line: usize) -> Vec<String> {
    let mut out = Vec::new();
    for raw in text.lines().skip(keyword_line) {
        let t = raw.trim();
        if raw.starts_with('[') && raw.trim_end().ends_with(']') {
            break;
        }
        out.push(t.to_string());
    }
    while out.first().is_some_and(String::is_empty) {
        out.remove(0);
    }
    out
}
