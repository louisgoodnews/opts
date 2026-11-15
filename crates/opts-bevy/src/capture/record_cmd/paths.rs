//! Disk checks and output paths of a new recording.

use opts_cli::protocol::RpcError;

use super::super::error::CaptureError;
use super::super::jobs::CaptureLimits;
use super::super::recording::RecFormat;
use super::super::storage::{self, CaptureState};

/// Refuses to start when the disk has less free space than the recording limits require.
pub(super) fn check_free_disk(
    state: &CaptureState,
    root: &std::path::Path,
    limits: &CaptureLimits,
) -> Result<(), RpcError> {
    if let Some(free) = (state.free_space)(root) {
        let need = limits.min_free_disk_mb * 1024 * 1024;
        if free < need {
            return Err(CaptureError::Failed(format!(
                "not enough free disk space for a recording: {} MiB free, at least {} MiB required",
                free / (1024 * 1024),
                limits.min_free_disk_mb
            ))
            .into());
        }
    }
    Ok(())
}

/// The output path and the sidecar path of a recording named `stem`.
pub(super) fn output_paths(
    root: &std::path::Path,
    stem: String,
    format: RecFormat,
) -> Result<(std::path::PathBuf, std::path::PathBuf), RpcError> {
    let paths = match format {
        RecFormat::Av1 => {
            let file = if stem.ends_with(".ivf") {
                stem
            } else {
                format!("{stem}.ivf")
            };
            let out = storage::resolve_name(root, &file)?;
            let side = super::super::screenshot::sidecar_path(&out);
            (out, side)
        }
        RecFormat::PngSequence => {
            let out = storage::resolve_dir(root, &format!("{stem}_rec"))?;
            let side = out.join("manifest.json");
            (out, side)
        }
    };
    Ok(paths)
}
