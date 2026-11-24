//! Strongly typed error definitions for Microsoft DirectX `.x` mesh parsing.
//!
//! Cites format specifications in `docs/src/formats/binary/x.md`.

use thiserror::Error;

/// Strongly typed errors that can occur while parsing a DirectX `.x` mesh file.
#[derive(Debug, Error, PartialEq)]
pub enum XParseError {
    /// Input buffer is too short to contain the mandatory 16-byte header.
    #[error("truncated header: expected 16 bytes, found {0}")]
    TruncatedHeader(usize),

    /// File magic did not match the required `xof ` signature.
    #[error("invalid magic: expected b\"xof \", found {0:?}")]
    InvalidMagic([u8; 4]),

    /// Version is not supported (only `0302` and `0303` are supported in OMSI).
    #[error("unsupported DirectX version: {0}")]
    UnsupportedVersion(String),

    /// Format is not supported (only text `txt ` is supported; binary `bin ` rejected).
    #[error("unsupported DirectX format: {0}")]
    UnsupportedFormat(String),

    /// Float size is not supported (only `0032` and `0064` are supported).
    #[error("unsupported float size: {0}")]
    UnsupportedFloatSize(String),

    /// Unexpected end of input while lexing or parsing.
    #[error("unexpected end of input at byte offset {0}")]
    UnexpectedEof(usize),

    /// Syntax error when encountering an unexpected token.
    #[error("syntax error at byte offset {offset}: expected {expected}, found {found}")]
    UnexpectedToken {
        /// Byte offset where the syntax error was encountered.
        offset: usize,
        /// Description of the expected token.
        expected: String,
        /// The unexpected token that was encountered.
        found: String,
    },

    /// Failed to parse a numeric integer literal.
    #[error("invalid integer at byte offset {offset}: {value}")]
    InvalidInteger {
        /// Byte offset of the integer literal.
        offset: usize,
        /// Raw token value.
        value: String,
    },

    /// Failed to parse a floating-point literal.
    #[error("invalid float at byte offset {offset}: {value}")]
    InvalidFloat {
        /// Byte offset of the float literal.
        offset: usize,
        /// Raw token value.
        value: String,
    },

    /// Face index refers to a vertex outside the bounds of the vertex buffer.
    #[error("face vertex index {index} out of bounds (vertex count {vertex_count})")]
    IndexOutOfBounds {
        /// Out-of-bounds vertex index.
        index: u32,
        /// Total number of available vertices.
        vertex_count: usize,
    },

    /// A named material reference `{ MaterialName }` could not be resolved.
    #[error("unresolved material reference '{0}'")]
    UnresolvedMaterialReference(String),

    /// Face polygon contains an unsupported number of vertices (must be 3 or 4).
    #[error("unsupported face vertex count: {0} (only triangles and quads supported)")]
    UnsupportedFaceVertexCount(u32),

    /// Normal count mismatch or indexing failure.
    #[error("invalid normal count: {0}")]
    InvalidNormalCount(usize),

    /// Element count declared in header differs from parsed elements.
    #[error("count mismatch: declared {declared}, parsed {parsed}")]
    CountMismatch {
        /// Count declared in header or block tag.
        declared: usize,
        /// Count actually parsed from body.
        parsed: usize,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = XParseError::TruncatedHeader(10);
        assert_eq!(
            err.to_string(),
            "truncated header: expected 16 bytes, found 10"
        );

        let err = XParseError::InvalidMagic(*b"ABCD");
        assert_eq!(
            err.to_string(),
            "invalid magic: expected b\"xof \", found [65, 66, 67, 68]"
        );

        let err = XParseError::UnsupportedFormat("bin ".to_string());
        assert_eq!(err.to_string(), "unsupported DirectX format: bin ");
    }
}
