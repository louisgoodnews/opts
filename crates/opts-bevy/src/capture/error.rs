//! Errors of the capture subsystem and their mapping to control protocol codes.

use opts_cli::protocol::{code, RpcError};
use serde_json::json;

/// Why a capture could not be taken or stored.
#[derive(Debug, thiserror::Error)]
pub enum CaptureError {
    /// A parameter is missing, mistyped or out of range (`-32602`).
    #[error("{0}")]
    InvalidParams(String),
    /// A name or path would leave the captures directory (`1002`).
    #[error("{0}")]
    Forbidden(String),
    /// There is no renderer, or the window shows nothing right now (`1001`).
    #[error("{0}")]
    NotReady(String),
    /// The frame did not arrive in time (`1005`).
    #[error("{0}")]
    Timeout(String),
    /// Another capture job is running (`1006`).
    #[error("{0}")]
    Busy(String),
    /// Encoding or writing failed (`1008`).
    #[error("{0}")]
    Failed(String),
}

impl CaptureError {
    /// A failed write of `what` to `path`.
    #[must_use]
    pub fn io(what: &str, path: &std::path::Path, e: &std::io::Error) -> Self {
        Self::Failed(format!("cannot {what} {}: {e}", path.display()))
    }
}

impl From<CaptureError> for RpcError {
    fn from(e: CaptureError) -> Self {
        let c = match &e {
            CaptureError::InvalidParams(_) => code::INVALID_PARAMS,
            CaptureError::Forbidden(_) => code::FORBIDDEN_PATH,
            CaptureError::NotReady(_) => code::NOT_READY,
            CaptureError::Timeout(_) => code::TIMEOUT,
            CaptureError::Busy(_) => code::BUSY,
            CaptureError::Failed(_) => code::FAILED,
        };
        let mut err = RpcError::new(c, e.to_string());
        if matches!(e, CaptureError::Forbidden(_)) {
            err = err.with_data(json!({ "hint": "captures are written below the captures directory; pass `name` as a relative path" }));
        }
        err
    }
}
