//! Validation of the `capture.record.start` parameters into a [`StartPlan`].

use opts_cli::protocol::RpcError;
use serde_json::Value;

use crate::control::params::{opt_f64, opt_str, opt_u64};

use super::super::jobs::CaptureLimits;
use super::super::recording::{RecFormat, StartPlan};
use super::DEFAULT_SCALE;

pub(super) fn parse(params: &Value, limits: &CaptureLimits) -> Result<StartPlan, RpcError> {
    let format = match opt_str(params, "format")? {
        None | Some("av1") => RecFormat::Av1,
        Some("png_sequence") => RecFormat::PngSequence,
        Some("h264") => {
            return Err(RpcError::invalid_params(
                "format `h264` needs a build with the optional `capture-h264` feature, which this build does not have; use `av1` (remux with ffmpeg for MP4)",
            ))
        }
        Some(other) => {
            return Err(RpcError::invalid_params(format!(
                "unknown format `{other}`; valid: av1, png_sequence"
            )))
        }
    };
    let fps = opt_u64(params, "fps")?.unwrap_or(30);
    if !(1..=60).contains(&fps) {
        return Err(RpcError::invalid_params("`fps` must be 1..=60"));
    }
    let duration_ms = opt_u64(params, "duration_ms")?.unwrap_or(limits.default_duration_ms);
    if !(1..=limits.max_duration_ms).contains(&duration_ms) {
        return Err(RpcError::invalid_params(format!(
            "`duration_ms` must be 1..={} (30 minutes)",
            limits.max_duration_ms
        )));
    }
    let size_mb = opt_u64(params, "max_size_mb")?.unwrap_or(limits.default_size_mb);
    if !(1..=limits.max_size_mb).contains(&size_mb) {
        return Err(RpcError::invalid_params(format!(
            "`max_size_mb` must be 1..={}",
            limits.max_size_mb
        )));
    }
    let quality = match opt_u64(params, "quality")? {
        Some(q @ 1..=100) => q as u8,
        Some(_) => return Err(RpcError::invalid_params("`quality` must be 1..=100")),
        None => 70,
    };
    let scale = opt_f64(params, "scale")?.unwrap_or(DEFAULT_SCALE);
    if !(0.25..=1.0).contains(&scale) {
        return Err(RpcError::invalid_params("`scale` must be 0.25..=1.0"));
    }
    Ok(StartPlan {
        format,
        fps: fps as u32,
        duration_ms,
        max_bytes: size_mb * 1024 * 1024,
        quality,
        scale: scale as f32,
    })
}
