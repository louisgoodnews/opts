//! Error types for map manifest and tile parsing.

use std::fmt;

/// Errors that can occur when parsing map manifests (`global.cfg`), tile layouts (`tile_*.map`),
/// and companion terrain/water files.
#[derive(Debug, Clone, PartialEq)]
pub enum MapParseError {
    /// The input bytes could not be decoded.
    DecodeError(String),
    /// A numeric parameter could not be parsed.
    InvalidNumber {
        /// The field name or description.
        field: &'static str,
        /// The unparseable string.
        value: String,
        /// 1-based line number.
        line: usize,
    },
    /// A required block or parameter is missing.
    MissingParameter {
        /// The keyword or field name.
        keyword: &'static str,
        /// Description of what was missing.
        detail: String,
        /// 1-based line number.
        line: usize,
    },
    /// Binary terrain/water file length or magic is invalid.
    InvalidBinary {
        /// Format description.
        format: &'static str,
        /// Detail of the error.
        detail: String,
    },
    /// An unexpected keyword format or structure.
    InvalidFormat {
        /// Description of the error.
        detail: String,
        /// 1-based line number.
        line: usize,
    },
}

impl fmt::Display for MapParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DecodeError(msg) => write!(f, "map decode error: {msg}"),
            Self::InvalidNumber { field, value, line } => {
                write!(f, "invalid number for {field} '{value}' at line {line}")
            }
            Self::MissingParameter {
                keyword,
                detail,
                line,
            } => {
                write!(
                    f,
                    "missing parameter for [{keyword}] at line {line}: {detail}"
                )
            }
            Self::InvalidBinary { format, detail } => {
                write!(f, "invalid binary {format}: {detail}")
            }
            Self::InvalidFormat { detail, line } => {
                write!(f, "invalid format at line {line}: {detail}")
            }
        }
    }
}

impl std::error::Error for MapParseError {}
