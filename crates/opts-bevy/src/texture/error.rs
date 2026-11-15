//! The error type of texture decoding.

use thiserror::Error;

/// Errors occurring during texture format sniffing or image decoding.
#[derive(Debug, Error)]
pub enum TextureError {
    /// Buffer is empty.
    #[error("texture buffer is empty")]
    EmptyBuffer,

    /// Unrecognized or unsupported image container magic bytes.
    #[error("unknown or unsupported image container format")]
    UnknownFormat,

    /// Error decoding DirectDraw Surface (.dds).
    #[error("failed to decode DDS: {0}")]
    DdsDecode(String),

    /// Error decoding generic image format.
    #[error("failed to decode image: {0}")]
    ImageDecode(String),

    /// Texture format or channel layout is unsupported.
    #[error("unsupported texture format: {0}")]
    UnsupportedFormat(String),
}
