//! Error types for vehicle configuration formats (`.bus`, `paths.cfg`).

use thiserror::Error;

/// Errors that can occur when parsing passenger path network configurations (`paths.cfg`).
#[derive(Debug, Error, PartialEq)]
pub enum PathsParseError {
    /// A keyword block did not contain the required number of parameters.
    #[error(
        "Keyword [{keyword}] at line {line} requires {expected} parameters, but found {found}"
    )]
    MissingParameters {
        /// Keyword name.
        keyword: String,
        /// Source line number.
        line: usize,
        /// Expected minimum count.
        expected: usize,
        /// Actual count found.
        found: usize,
    },

    /// A numeric coordinate or index could not be parsed.
    #[error("Failed to parse number at line {line}: {value}")]
    InvalidNumber {
        /// Source line number.
        line: usize,
        /// Parameter value.
        value: String,
    },
}

/// Errors that can occur when parsing vehicle manifests (`.bus`).
#[derive(Debug, Error, PartialEq)]
pub enum BusParseError {
    /// A keyword block did not contain the required number of parameters.
    #[error(
        "Keyword [{keyword}] at line {line} requires {expected} parameters, but found {found}"
    )]
    MissingParameters {
        /// Keyword name.
        keyword: String,
        /// Source line number.
        line: usize,
        /// Expected minimum count.
        expected: usize,
        /// Actual count found.
        found: usize,
    },

    /// A numeric parameter could not be parsed.
    #[error("Failed to parse number at line {line}: {value}")]
    InvalidNumber {
        /// Source line number.
        line: usize,
        /// Parameter value.
        value: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = PathsParseError::MissingParameters {
            keyword: "pathpnt".to_string(),
            line: 10,
            expected: 3,
            found: 1,
        };
        assert_eq!(
            err.to_string(),
            "Keyword [pathpnt] at line 10 requires 3 parameters, but found 1"
        );
    }
}
