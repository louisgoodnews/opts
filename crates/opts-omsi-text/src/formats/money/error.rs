//! Error types for parsing `Money/<pack>/*.cfg` money system configurations.

use thiserror::Error;

/// Errors that can occur when parsing a money system configuration file.
#[derive(Debug, Error, PartialEq)]
pub enum MoneyParseError {
    /// The configuration file did not define a `[currency]` block.
    #[error("Missing [currency] definition in money system configuration")]
    MissingCurrency,

    /// A required parameter line was missing for a keyword block.
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        assert_eq!(
            MoneyParseError::MissingCurrency.to_string(),
            "Missing [currency] definition in money system configuration"
        );
        let err = MoneyParseError::MissingParameters {
            keyword: "coin".to_string(),
            line: 42,
            expected: 2,
            found: 1,
        };
        assert_eq!(
            err.to_string(),
            "Keyword [coin] at line 42 requires 2 parameters, but found 1"
        );
    }
}
