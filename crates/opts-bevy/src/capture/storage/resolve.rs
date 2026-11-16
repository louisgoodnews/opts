//! Resolving names below the captures root: creating directories and refusing symlinks that leave it.

use std::path::{Path, PathBuf};

use super::super::error::CaptureError;
use super::names::{forbidden, validate_relative};

/// Creates the directories of `rel` below `root`, checking every existing step stays inside the
/// canonical `root` (a symlink that leaves it is refused before anything is created through it).
fn create_dirs_inside(root: &Path, rel: &Path, name: &str) -> Result<PathBuf, CaptureError> {
    let mut cur = root.to_path_buf();
    for seg in rel.components() {
        let next = cur.join(seg);
        if next.exists() {
            let real = next
                .canonicalize()
                .map_err(|e| CaptureError::io("resolve", &next, &e))?;
            if !real.starts_with(root) {
                return Err(forbidden(name, "it leaves the captures directory"));
            }
            cur = real;
        } else {
            std::fs::create_dir(&next).map_err(|e| CaptureError::io("create", &next, &e))?;
            cur = next;
        }
    }
    Ok(cur)
}

/// Resolves a relative `name` below `root`, creating its parent directories.
///
/// # Errors
/// `Forbidden` when the name is not a plain relative path or a symlink inside the root leads out
/// of it, `Failed` when a directory cannot be created.
pub fn resolve_name(root: &Path, name: &str) -> Result<PathBuf, CaptureError> {
    let rel = validate_relative(name)?;
    let root = root
        .canonicalize()
        .map_err(|e| CaptureError::io("resolve", root, &e))?;
    let file = rel
        .file_name()
        .ok_or_else(|| forbidden(name, "it has no file name"))?
        .to_owned();
    let parent = create_dirs_inside(&root, rel.parent().unwrap_or(Path::new("")), name)?;
    Ok(parent.join(file))
}

/// Resolves a relative directory `name` below `root` and creates it.
///
/// # Errors
/// As [`resolve_name`].
pub fn resolve_dir(root: &Path, name: &str) -> Result<PathBuf, CaptureError> {
    let rel = validate_relative(name)?;
    let root = root
        .canonicalize()
        .map_err(|e| CaptureError::io("resolve", root, &e))?;
    create_dirs_inside(&root, &rel, name)
}

/// Resolves an explicit `path` (the legacy form): its directory must exist and lie inside `root`.
///
/// # Errors
/// `Forbidden` outside the root, `Failed` when the directory does not exist.
pub fn resolve_explicit(root: &Path, path: &Path) -> Result<PathBuf, CaptureError> {
    let root = root
        .canonicalize()
        .map_err(|e| CaptureError::io("resolve", root, &e))?;
    let file = path
        .file_name()
        .ok_or_else(|| CaptureError::InvalidParams("path has no file name".into()))?;
    let dir = match path.parent() {
        Some(p) if !p.as_os_str().is_empty() => p.to_path_buf(),
        _ => std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
    };
    let real_dir = dir.canonicalize().map_err(|_| {
        CaptureError::Forbidden(format!(
            "{} does not exist or is outside the captures directory {}",
            dir.display(),
            root.display()
        ))
    })?;
    if real_dir.starts_with(&root) {
        Ok(real_dir.join(file))
    } else {
        Err(CaptureError::Forbidden(format!(
            "write outside the captures directory {}: {}",
            root.display(),
            path.display()
        )))
    }
}
