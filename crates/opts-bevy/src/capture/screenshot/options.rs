//! The options of `screenshot`: the still formats, the target and the parameter parsing.

use std::path::PathBuf;

use opts_cli::protocol::RpcError;
use serde_json::Value;

use crate::control::params::{opt_bool, opt_str, opt_u64};
use super::DEFAULT_JPEG_QUALITY;
use super::super::error::CaptureError;

/// Still image formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StillFormat {
    /// Lossless, the default.
    Png,
    /// Lossy, for size.
    Jpeg,
}

impl StillFormat {
    /// Parses the `format` parameter.
    ///
    /// # Errors
    /// `InvalidParams` naming the valid values.
    pub fn parse(s: &str) -> Result<Self, CaptureError> {
        match s {
            "png" => Ok(Self::Png),
            "jpeg" | "jpg" => Ok(Self::Jpeg),
            other => Err(CaptureError::InvalidParams(format!(
                "unknown format `{other}`; valid: png, jpeg"
            ))),
        }
    }

    /// Format name as reported in results and metadata.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpeg => "jpeg",
        }
    }

    /// File extension.
    #[must_use]
    pub const fn ext(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpeg => "jpg",
        }
    }
}

/// Where a still goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    /// A generated name in the captures directory.
    Generated,
    /// A relative name below the captures directory (`name`).
    Name(String),
    /// An explicit path that must lie inside the captures directory (`path`).
    Path(PathBuf),
}

/// Parsed parameters of `screenshot`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Options {
    /// Destination.
    pub target: Target,
    /// Still format.
    pub format: StillFormat,
    /// JPEG quality (1..=100).
    pub quality: u8,
    /// Whether to write the sidecar and return the metadata.
    pub include_metadata: bool,
}

/// Reads `path`, `name`, `format`, `quality` and `include_metadata`.
///
/// # Errors
/// `InvalidParams` for mistyped or conflicting parameters.
pub fn parse_options(params: &Value) -> Result<Options, RpcError> {
    let path = opt_str(params, "path")?;
    let name = opt_str(params, "name")?;
    let target = match (path, name) {
        (Some(_), Some(_)) => {
            return Err(RpcError::invalid_params(
                "`path` and `name` are mutually exclusive; prefer `name`",
            ))
        }
        (Some(p), None) => Target::Path(PathBuf::from(p)),
        (None, Some(n)) => Target::Name(n.to_string()),
        (None, None) => Target::Generated,
    };
    let format = match opt_str(params, "format")? {
        Some(f) => StillFormat::parse(f)?,
        None => StillFormat::Png,
    };
    let quality = match opt_u64(params, "quality")? {
        Some(q @ 1..=100) => q as u8,
        Some(_) => return Err(RpcError::invalid_params("`quality` must be 1..=100")),
        None => DEFAULT_JPEG_QUALITY,
    };
    Ok(Options {
        target,
        format,
        quality,
        include_metadata: opt_bool(params, "include_metadata")?.unwrap_or(true),
    })
}
