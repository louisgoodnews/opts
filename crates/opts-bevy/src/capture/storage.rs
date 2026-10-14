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

/// Resolves a relative `name` below `root` (created if missing), creating its parent directories.
///
/// # Errors
/// `Forbidden` when the name is not a plain relative path or a symlink inside the root leads out
/// of it, `Failed` when a directory cannot be created.
pub fn resolve_name(root: &Path, name: &str) -> Result<PathBuf, CaptureError> {
    let rel = validate_relative(name)?;
    let root = root
        .canonicalize()
        .map_err(|e| CaptureError::io("resolve", root, &e))?;
    let target = root.join(&rel);
    let parent = target.parent().unwrap_or(&root);
    std::fs::create_dir_all(parent).map_err(|e| CaptureError::io("create", parent, &e))?;
    let real_parent = parent
        .canonicalize()
        .map_err(|e| CaptureError::io("resolve", parent, &e))?;
    if !real_parent.starts_with(&root) {
        return Err(forbidden(name, "it leaves the captures directory"));
    }
    let file = target
        .file_name()
        .ok_or_else(|| forbidden(name, "it has no file name"))?;
    Ok(real_parent.join(file))
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
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("opts_capstore_{name}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).expect("temp dir");
        p
    }

    #[test]
    fn traversal_and_absolute_names_are_rejected() {
        for bad in [
            "../../etc/passwd",
            "/etc/passwd",
            "a/../../b",
            "./x.png",
            "a//b/../../../c",
            "",
            "dir/",
            "C:\\x.png",
            "x\u{0}y",
        ] {
            let r = validate_relative(bad);
            // A drive prefix is only absolute on Windows; elsewhere it is one plain segment, and
            // `resolve_name` then keeps it inside the root like any other name.
            if bad.starts_with("C:") && cfg!(unix) {
                continue;
            }
            assert!(matches!(r, Err(CaptureError::Forbidden(_))), "{bad:?}");
        }
        assert_eq!(
            validate_relative("run1/shot.png").expect("plain"),
            PathBuf::from("run1").join("shot.png")
        );
    }

    #[test]
    fn names_resolve_inside_the_root_and_create_parents() {
        let root = temp("a");
        let p = resolve_name(&root, "run1/deep/shot.png").expect("inside");
        assert!(p.starts_with(root.canonicalize().expect("root")));
        assert!(p.parent().expect("parent").is_dir());
    }

    #[cfg(unix)]
    #[test]
    fn a_symlink_leading_out_of_the_root_is_rejected() {
        let (root, outside) = (temp("b"), temp("c"));
        std::os::unix::fs::symlink(&outside, root.join("link")).expect("symlink");
        let err = resolve_name(&root, "link/shot.png").unwrap_err();
        assert!(matches!(err, CaptureError::Forbidden(_)), "{err}");
    }

    #[test]
    fn explicit_paths_must_be_inside_the_root() {
        let (root, other) = (temp("d"), temp("e"));
        assert!(resolve_explicit(&root, &root.join("a.png")).is_ok());
        assert!(matches!(
            resolve_explicit(&root, &other.join("a.png")),
            Err(CaptureError::Forbidden(_))
        ));
        assert!(matches!(
            resolve_explicit(&root, Path::new("/etc/passwd")),
            Err(CaptureError::Forbidden(_))
        ));
    }

    #[test]
    fn generated_stems_are_unique_and_carry_the_session() {
        let mut s = CaptureState::new(None);
        let (a, b) = (s.next_stem(), s.next_stem());
        assert_ne!(a, b);
        assert!(a.contains(&s.session_id));
        assert!(a.ends_with("_0001") && b.ends_with("_0002"));
    }

    #[test]
    fn atomic_write_leaves_no_part_file() {
        let root = temp("f");
        let p = root.join("x.bin");
        write_atomic(&p, b"abc").expect("write");
        assert_eq!(std::fs::read(&p).expect("read"), b"abc");
        assert!(!root.join("x.bin.part").exists());
    }
}
