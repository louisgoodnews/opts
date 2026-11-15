//! Limits and request options of `capture.burst`.

use opts_cli::protocol::RpcError;
use serde_json::Value;

use crate::control::params::{opt_bool, opt_str, opt_u64};
use super::super::screenshot::{StillFormat, DEFAULT_JPEG_QUALITY};

/// Most frames in one burst.
pub const MAX_COUNT: u64 = 300;
/// Shortest interval (one frame at 60 Hz).
pub const MIN_INTERVAL_MS: u64 = 16;
/// Longest interval.
pub const MAX_INTERVAL_MS: u64 = 60_000;
/// A burst answers within the control server's 120 s limit for deferred replies.
pub const MAX_TOTAL_MS: u64 = 90_000;

/// The count, the interval and their product must stay inside the limits.
pub(super) fn check_limits(count: u64, interval_ms: u64) -> Result<(), RpcError> {
    if !(1..=MAX_COUNT).contains(&count) {
        return Err(RpcError::invalid_params(format!(
            "`count` must be 1..={MAX_COUNT}"
        )));
    }
    if !(MIN_INTERVAL_MS..=MAX_INTERVAL_MS).contains(&interval_ms) {
        return Err(RpcError::invalid_params(format!(
            "`interval_ms` must be {MIN_INTERVAL_MS}..={MAX_INTERVAL_MS}"
        )));
    }
    if count * interval_ms > MAX_TOTAL_MS {
        return Err(RpcError::invalid_params(format!(
            "`count` x `interval_ms` may not exceed {MAX_TOTAL_MS} ms (the reply must arrive within the control server's deferred-reply limit)"
        )));
    }
    Ok(())
}

/// The format, JPEG quality, metadata flag and folder name of the request.
pub(super) fn still_options(params: &Value) -> Result<(StillFormat, u8, bool, Option<String>), RpcError> {
    let format = match opt_str(params, "format")? {
        Some(f) => StillFormat::parse(f)?,
        None => StillFormat::Png,
    };
    let quality = match opt_u64(params, "quality")? {
        Some(q @ 1..=100) => q as u8,
        Some(_) => return Err(RpcError::invalid_params("`quality` must be 1..=100")),
        None => DEFAULT_JPEG_QUALITY,
    };
    let with_meta = opt_bool(params, "metadata")?.unwrap_or(true);
    let dir_name = opt_str(params, "dir")?.map(str::to_string);
    Ok((format, quality, with_meta, dir_name))
}
