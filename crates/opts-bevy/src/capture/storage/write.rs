//! Atomic writes: the bytes go to a temporary sibling file that is renamed into place.

use std::path::{Path, PathBuf};

use super::super::error::CaptureError;

/// Writes `bytes` to `path` through a temporary sibling and a rename, so a reader never sees a
/// half-written file.
///
/// # Errors
/// `Failed` with the I/O cause.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), CaptureError> {
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".part");
    let tmp = PathBuf::from(tmp);
    std::fs::write(&tmp, bytes).map_err(|e| CaptureError::io("write", &tmp, &e))?;
    std::fs::rename(&tmp, path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        CaptureError::io("write", path, &e)
    })
}
