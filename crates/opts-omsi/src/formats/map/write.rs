//! Writers for map text files: `global.cfg` ([`MapManifest::to_text`]) and the `[object]`
//! records of a tile ([`MapTile::to_text`]).
//!
//! The output is what the parsers in this module read back: `parse(write(x)) == x` for every
//! value the writers support. OMSI's text scanner drops blank lines and comment lines and trims
//! every line, so a value that is empty, has surrounding white space, spans several lines or
//! looks like a keyword or a comment cannot be written; the writers reject it with
//! [`MapWriteError`] instead of producing a file that parses differently.
//!
//! See `docs/src/formats/map/global-cfg.md` and `docs/src/formats/map/tiles.md`.

use std::fmt::Write as _;

use super::{MapManifest, MapObjectInstance, MapTile};

/// Why a value cannot be written as one line of an OMSI text file.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum MapWriteError {
    /// The value would not survive the text scanner (empty, multi-line, padded, keyword or comment).
    #[error("{field}: value {value:?} cannot be written as one OMSI text line ({reason})")]
    UnwritableLine {
        /// Which field the value belongs to.
        field: &'static str,
        /// The offending value.
        value: String,
        /// What is wrong with it.
        reason: &'static str,
    },
    /// The tile has records the writer does not support (splines, rules, attachments, flags).
    #[error("tile writer supports only [version] and [object] records; found {0}")]
    UnsupportedTileContent(&'static str),
}

/// Checks that `value` reads back unchanged as one parameter line.
fn line(field: &'static str, value: &str) -> Result<String, MapWriteError> {
    let reject = |reason| {
        Err(MapWriteError::UnwritableLine {
            field,
            value: value.to_string(),
            reason,
        })
    };
    if value.is_empty() {
        return reject("empty lines are dropped by the scanner");
    }
    if value.contains(['\n', '\r']) {
        return reject("contains a line break");
    }
    if value.trim() != value {
        return reject("leading or trailing white space is trimmed");
    }
    if value.starts_with("//") || value.starts_with('#') {
        return reject("comment lines are dropped");
    }
    if value.starts_with('[') && value.ends_with(']') {
        return reject("reads as a keyword");
    }
    Ok(value.to_string())
}

/// Checks that a string value reads back unchanged. The tile reader takes string values line by
/// line, so blank and comment-like values survive; padding, line breaks and keyword lines do not.
fn string_value(field: &'static str, value: &str) -> Result<String, MapWriteError> {
    if value.is_empty() || value.starts_with("//") || value.starts_with('#') {
        if value.contains(['\n', '\r']) {
            return line(field, value);
        }
        if value.trim() != value {
            return line(field, value);
        }
        return Ok(value.to_string());
    }
    line(field, value)
}

/// Appends a keyword followed by its parameter lines.
fn block(out: &mut String, keyword: &str, params: &[String]) {
    let _ = writeln!(out, "[{keyword}]");
    for p in params {
        let _ = writeln!(out, "{p}");
    }
    out.push('\n');
}

fn object_lines(obj: &MapObjectInstance, legacy: bool) -> Result<Vec<String>, MapWriteError> {
    let mut p = vec![
        obj.flag.to_string(),
        line("object sco", &obj.sco_file)?,
        obj.id_code.to_string(),
    ];
    p.extend(
        [
            obj.position.x,
            obj.position.y,
            obj.position.z,
            obj.heading_deg,
        ]
        .map(|v| v.to_string()),
    );
    if !legacy {
        p.push(obj.pitch_deg.unwrap_or(0.0).to_string());
        p.push(obj.roll_deg.unwrap_or(0.0).to_string());
    }
    p.push(obj.string_vars.len().to_string());
    for s in &obj.string_vars {
        p.push(string_value("object string variable", s)?);
    }
    Ok(p)
}

mod manifest;
mod tile;

#[cfg(test)]
mod tests;
