//! Error types for `.o3d` parsing.
//!
//! Corresponds to the error reporting requirements in
//! `docs/src/formats/binary/o3d.md`.

/// Result type alias for `.o3d` parser operations.
pub type Result<T> = std::result::Result<T, O3dParseError>;

/// Error variants encountered when parsing an `.o3d` byte stream.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum O3dParseError {
    /// The file does not start with the required magic bytes `[0x84, 0x19]`.
    #[error("invalid magic: expected [0x84, 0x19], got [0x{0:02X}, 0x{1:02X}]")]
    InvalidMagic(u8, u8),

    /// The byte slice ended prematurely while expecting more data.
    #[error("file truncated: expected at least {expected} bytes, got {actual}")]
    Truncated {
        /// Expected minimum number of bytes.
        expected: usize,
        /// Actual available number of bytes.
        actual: usize,
    },

    /// An unrecognized chunk tag or ID was encountered at the given byte offset.
    #[error("unknown chunk id: 0x{0:08X} at offset {1}")]
    UnknownChunk(u32, usize),

    /// A string inside the file (e.g. texture or bone name) was not valid UTF-8/ASCII.
    #[error("invalid UTF-8 in string table at offset {0}")]
    InvalidUtf8(usize),

    /// The `.o3d` version byte is not in the supported set (1, 3, 4, 5, 7).
    #[error("unsupported o3d version: {0}")]
    UnsupportedVersion(u32),

    /// The vertex buffer size did not match the expected 32-byte stride.
    #[error("vertex format mismatch: expected {expected} bytes per vertex, got {actual}")]
    VertexFormatMismatch {
        /// Expected byte stride per vertex.
        expected: usize,
        /// Actual byte stride or byte count encountered.
        actual: usize,
    },

    /// A face index references a vertex index out of bounds.
    #[error("index out of bounds: {index} >= {count}")]
    IndexOutOfBounds {
        /// The face vertex index that was out of range.
        index: u32,
        /// The total number of vertices in the vertex buffer.
        count: usize,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = O3dParseError::Truncated {
            expected: 32,
            actual: 10,
        };
        assert_eq!(
            err.to_string(),
            "file truncated: expected at least 32 bytes, got 10"
        );

        let err = O3dParseError::UnsupportedVersion(99);
        assert_eq!(err.to_string(), "unsupported o3d version: 99");

        let err = O3dParseError::InvalidMagic(0x12, 0x34);
        assert_eq!(
            err.to_string(),
            "invalid magic: expected [0x84, 0x19], got [0x12, 0x34]"
        );

        let err = O3dParseError::IndexOutOfBounds {
            index: 10,
            count: 5,
        };
        assert_eq!(err.to_string(), "index out of bounds: 10 >= 5");
    }
}
