//! Error type for `.hum` parsing.

use std::fmt;

/// Errors that can occur when parsing a human configuration (`.hum`).
#[derive(Debug, Clone, PartialEq)]
pub enum HumanParseError {
    /// A mandatory keyword is absent.
    MissingKeyword(&'static str),
    /// A keyword has fewer parameter lines than required.
    TooFewParameters {
        /// The keyword.
        keyword: &'static str,
        /// Parameters required.
        expected: usize,
        /// Parameters found.
        found: usize,
        /// 1-based line number of the keyword.
        line: usize,
    },
    /// A parameter could not be parsed as a number.
    InvalidNumber {
        /// The keyword.
        keyword: &'static str,
        /// The offending text.
        value: String,
        /// 1-based line number of the keyword.
        line: usize,
    },
}

impl fmt::Display for HumanParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingKeyword(k) => write!(f, "missing mandatory keyword [{k}]"),
            Self::TooFewParameters {
                keyword,
                expected,
                found,
                line,
            } => write!(
                f,
                "[{keyword}] at line {line}: expected {expected} parameters, found {found}"
            ),
            Self::InvalidNumber {
                keyword,
                value,
                line,
            } => {
                write!(f, "[{keyword}] at line {line}: '{value}' is not a number")
            }
        }
    }
}

impl std::error::Error for HumanParseError {}
