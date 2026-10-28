//! Where captures go: the captures directory, name validation, default names and atomic writes.
//!
//! Every name that reaches the file system passes through [`resolve_name`] or
//! [`resolve_explicit`]; both guarantee the result is inside the canonical captures root.

use std::path::{Component, Path, PathBuf};

use bevy::prelude::Resource;

use super::error::CaptureError;

/// Longest accepted relative name in bytes.
const MAX_NAME_BYTES: usize = 200;

/// Session-wide capture bookkeeping: where captures go, the session id and the name counter.
#[derive(Resource, Debug, Clone)]
pub struct CaptureState {
    /// The captures directory; `None` when the platform has no data directory.
    pub root: Option<PathBuf>,
    /// Six hex digits identifying this process run, part of every generated name.
    pub session_id: String,
    /// Free bytes on the volume holding a path (tests replace it to simulate a full disk).
    pub free_space: fn(&Path) -> Option<u64>,
    counter: u32,
}

impl Default for CaptureState {
    fn default() -> Self {
        Self::new(default_root())
    }
}

impl CaptureState {
    /// State for the given captures directory.
    #[must_use]
    pub fn new(root: Option<PathBuf>) -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.subsec_nanos() ^ (d.as_secs() as u32));
        Self {
            root,
            session_id: format!("{:06x}", (nanos ^ std::process::id()) & 0x00ff_ffff),
            free_space: free_bytes,
            counter: 0,
        }
    }

    /// The next generated stem, `{datetime}_{session_id}_{counter}`.
    pub fn next_stem(&mut self) -> String {
        self.counter += 1;
        format!(
            "{}_{}_{:04}",
            chrono::Local::now().format("%Y%m%dT%H%M%S"),
            self.session_id,
            self.counter
        )
    }

    /// The captures root, created if missing.
    ///
    /// # Errors
    /// `Failed` when there is no data directory or it cannot be created.
    pub fn ensure_root(&self) -> Result<PathBuf, CaptureError> {
        let root = self
            .root
            .clone()
            .ok_or_else(|| CaptureError::Failed("there is no captures directory".into()))?;
        std::fs::create_dir_all(&root).map_err(|e| CaptureError::io("create", &root, &e))?;
        root.canonicalize()
            .map_err(|e| CaptureError::io("resolve", &root, &e))
    }
}

/// The default captures directory: `<user data dir>/opts/captures`.
#[must_use]
pub fn default_root() -> Option<PathBuf> {
    directories::BaseDirs::new().map(|d| d.data_dir().join("opts").join("captures"))
}

/// Free bytes on the volume that holds `path`, or `None` when it cannot be determined.
#[must_use]
pub fn free_bytes(path: &Path) -> Option<u64> {
    let real = path.canonicalize().ok()?;
    let disks = sysinfo::Disks::new_with_refreshed_list();
    disks
        .list()
        .iter()
        .filter(|d| real.starts_with(d.mount_point()))
        .max_by_key(|d| d.mount_point().as_os_str().len())
        .map(sysinfo::Disk::available_space)
}

fn forbidden(name: &str, why: &str) -> CaptureError {
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

#[cfg(test)]
mod tests;
