//! Error types for scenery object and spline parsing.

use std::fmt;

/// Errors that can occur when parsing scenery object (`.sco`) and spline (`.sli`) files.
#[derive(Debug, Clone, PartialEq)]
pub enum SceneryParseError {
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
    /// An unexpected keyword format or structure.
    InvalidFormat {
        /// Description of the error.
        detail: String,
        /// 1-based line number.
        line: usize,
    },
}

impl fmt::Display for SceneryParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DecodeError(msg) => write!(f, "scenery decode error: {msg}"),
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
            Self::InvalidFormat { detail, line } => {
                write!(f, "invalid format at line {line}: {detail}")
            }
        }
    }
}

impl std::error::Error for SceneryParseError {}
