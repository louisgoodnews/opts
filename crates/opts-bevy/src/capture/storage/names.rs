//! Names of captures: the lexical check of a relative name, which refuses anything that is not a
//! plain relative path.

use std::path::{Component, Path, PathBuf};

use super::super::error::CaptureError;

/// Longest accepted relative name in bytes.
const MAX_NAME_BYTES: usize = 200;

pub(super) fn forbidden(name: &str, why: &str) -> CaptureError {
    CaptureError::Forbidden(format!("`{name}` is not a valid capture name: {why}"))
}

/// Checks a relative name lexically: only plain segments, no `..`, no root or drive prefix.
///
/// # Errors
/// `Forbidden` for anything that is not a plain relative path.
pub fn validate_relative(name: &str) -> Result<PathBuf, CaptureError> {
    if name.is_empty() {
        return Err(forbidden(name, "it is empty"));
    }
    if name.len() > MAX_NAME_BYTES {
        return Err(forbidden(name, "it is too long"));
    }
    if name
        .chars()
        .any(|c| c.is_control() || c == '\\' && cfg!(unix))
    {
        return Err(forbidden(
            name,
            "it contains control or backslash characters",
        ));
    }
    let mut out = PathBuf::new();
    for comp in Path::new(name).components() {
        match comp {
            Component::Normal(seg) => out.push(seg),
            Component::CurDir => return Err(forbidden(name, "it contains a `.` segment")),
            Component::ParentDir => return Err(forbidden(name, "it contains `..`")),
            Component::RootDir | Component::Prefix(_) => {
                return Err(forbidden(name, "it is absolute"))
            }
        }
    }
    if out.as_os_str().is_empty() || name.ends_with('/') {
        return Err(forbidden(name, "it has no file name"));
    }
    Ok(out)
}
