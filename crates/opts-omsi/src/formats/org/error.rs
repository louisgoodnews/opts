//! Error types for parsing `.org` registration lists.

use thiserror::Error;

/// Errors that can occur when parsing an `.org` registration pool.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum OrgParseError {
    /// The pool contains no valid, non-empty registration entries.
    #[error("Registration pool is empty")]
    EmptyPool,

    /// An I/O or decoding error occurred.
    #[error("Failed to decode text: {0}")]
    DecodeError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        assert_eq!(
            OrgParseError::EmptyPool.to_string(),
            "Registration pool is empty"
        );
        assert_eq!(
            OrgParseError::DecodeError("invalid utf8".to_string()).to_string(),
            "Failed to decode text: invalid utf8"
        );
    }
}
