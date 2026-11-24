//! Error types for parsing vehicle model configuration files (`model.cfg`).

use thiserror::Error;

/// Errors that can occur when parsing a `model.cfg` file.
#[derive(Debug, Error, PartialEq)]
pub enum ModelParseError {
    /// A keyword block did not contain the required number of parameters.
    #[error(
        "Keyword [{keyword}] at line {line} requires {expected} parameters, but found {found}"
    )]
    MissingParameters {
        /// Keyword name.
        keyword: String,
        /// Source line number.
        line: usize,
        /// Expected minimum parameter count.
        expected: usize,
        /// Actual parameter count found.
        found: usize,
    },

    /// A numeric parameter could not be parsed.
    #[error("Failed to parse number at line {line}: {value}")]
    InvalidNumber {
        /// Source line number.
        line: usize,
        /// Literal text that failed to parse.
        value: String,
    },

    /// A 3D coordinate vector could not be parsed.
    #[error("Failed to parse 3D vector at line {line}: {message}")]
    InvalidVector {
        /// Source line number.
        line: usize,
        /// Descriptive message.
        message: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = ModelParseError::MissingParameters {
            keyword: "mesh".to_string(),
            line: 12,
            expected: 1,
            found: 0,
        };
        assert_eq!(
            err.to_string(),
            "Keyword [mesh] at line 12 requires 1 parameters, but found 0"
        );
    }
}
