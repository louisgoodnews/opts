//! Scanning a tile text into blocks and reading numbers from its lines.

use super::*;

/// A tile block: the keyword with its generic parameters (non-blank, non-comment lines, as
/// [`crate::formats::text::scan_blocks`] gives them) and every line up to the next keyword
/// verbatim (trimmed, blank lines included) for the records that carry string values.
pub(super) struct TileBlock {
    pub(super) block: OmsiBlock,
    pub(super) lines: Vec<String>,
}

impl TileBlock {
    pub(super) fn cursor(&self) -> LineCursor<'_> {
        LineCursor {
            lines: &self.lines,
            pos: 0,
        }
    }
}

/// Reads a record line by line: positional fields skip blank and comment lines (as the generic
/// scanner does), string values are taken literally, so a blank value stays a blank value instead
/// of pulling the next line (the `Object Nr. N` comment) into its place.
pub(super) struct LineCursor<'a> {
    lines: &'a [String],
    pos: usize,
}

impl<'a> LineCursor<'a> {
    /// Up to `n` positional fields.
    pub(super) fn fields(&mut self, n: usize) -> Vec<&'a str> {
        let mut out = Vec::with_capacity(n);
        while out.len() < n && self.pos < self.lines.len() {
            let l = self.lines[self.pos].as_str();
            self.pos += 1;
            if !(l.is_empty() || l.starts_with("//") || l.starts_with('#')) {
                out.push(l);
            }
        }
        out
    }

    /// The `count` string values that follow (fewer when the record is cut short).
    pub(super) fn strings(&mut self, count: Option<&str>) -> Vec<String> {
        let n = count
            .and_then(|c| c.trim().parse::<usize>().ok())
            .unwrap_or(0);
        let end = (self.pos + n).min(self.lines.len());
        let out = self.lines[self.pos..end].to_vec();
        self.pos = end;
        out
    }
}

/// Splits tile text into [`TileBlock`]s (keyword lines as in `scan_blocks`).
pub(super) fn scan_tile_blocks(text: &str) -> Vec<TileBlock> {
    let mut out: Vec<TileBlock> = Vec::new();
    for (i, raw_line) in text.lines().enumerate() {
        let line = raw_line.trim_end_matches(['\r', '\n']).trim();
        if !line.is_empty() && raw_line.starts_with('[') && raw_line.trim_end().ends_with(']') {
            if let Some(kw) = parse_keyword(raw_line.trim_end()) {
                out.push(TileBlock {
                    block: OmsiBlock {
                        keyword: kw.to_ascii_lowercase(),
                        line_number: i + 1,
                        parameters: Vec::new(),
                    },
                    lines: Vec::new(),
                });
                continue;
            }
        }
        let Some(current) = out.last_mut() else {
            continue;
        };
        if !(line.is_empty() || line.starts_with("//") || line.starts_with('#')) {
            current.block.parameters.push(line.to_string());
        }
        current.lines.push(line.to_string());
    }
    out
}

pub(super) fn parse_i64(s: &str, field: &'static str, line: usize) -> Result<i64, MapParseError> {
    s.trim()
        .parse::<i64>()
        .map_err(|_| MapParseError::InvalidNumber {
            field,
            value: s.to_string(),
            line,
        })
}

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

/// A connection id: `0` means unconnected, and some editors write `-1` for the same thing (all
/// 61 tiles of Neuendorf, Rheinhausen and Ahlheim 5 that failed to parse did).
pub(super) fn parse_conn(s: &str, field: &'static str, line: usize) -> Result<u64, MapParseError> {
    match s.trim().parse::<i64>() {
        Ok(v) if v < 0 => Ok(0),
        _ => parse_u64(s, field, line),
    }
}

pub(super) fn parse_u64(s: &str, field: &'static str, line: usize) -> Result<u64, MapParseError> {
    s.trim()
        .parse::<u64>()
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
