//! Filesystem policy of the control server: reads below the OMSI root or an allowed directory,
//! writes below the reports directory or an allowed directory. Paths are canonicalised first, so
//! `..` segments and symlinks cannot escape.

use std::path::{Path, PathBuf};

use bevy::prelude::Resource;
use opts_cli::protocol::{code, RpcError};

/// Allowed roots.
#[derive(Resource, Debug, Clone, Default)]
pub struct PathPolicy {
    /// Directories commands may read from.
    pub read_roots: Vec<PathBuf>,
    /// Directories commands may write to.
    pub write_roots: Vec<PathBuf>,
}

fn forbidden(path: &Path, what: &str) -> RpcError {
    RpcError::new(
        code::FORBIDDEN_PATH,
        format!("{what} outside the allowed directories: {}", path.display()),
    )
}

fn within(path: &Path, roots: &[PathBuf]) -> bool {
    roots
        .iter()
        .filter_map(|r| r.canonicalize().ok())
        .any(|r| path.starts_with(r))
}

impl PathPolicy {
    /// Resolves a path for reading.
    ///
    /// # Errors
    /// `1004` when it does not exist, `1002` when it is outside the read roots.
    pub fn check_read(&self, path: &Path) -> Result<PathBuf, RpcError> {
        let real = path.canonicalize().map_err(|_| {
            RpcError::new(
                code::NOT_FOUND,
                format!("{} does not exist", path.display()),
            )
        })?;
        if within(&real, &self.read_roots) || within(&real, &self.write_roots) {
            Ok(real)
        } else {
            Err(forbidden(path, "read"))
        }
    }

    /// Resolves a path for writing (the file itself may not exist yet; its directory must).
    ///
    /// # Errors
    /// `1004` when the directory does not exist, `1002` when it is outside the write roots.
    pub fn check_write(&self, path: &Path) -> Result<PathBuf, RpcError> {
        let name = path
            .file_name()
            .ok_or_else(|| RpcError::invalid_params("path has no file name"))?;
        let dir = match path.parent() {
            Some(p) if !p.as_os_str().is_empty() => p,
            _ => Path::new("."),
        };
        let real_dir = dir.canonicalize().map_err(|_| {
            RpcError::new(code::NOT_FOUND, format!("{} does not exist", dir.display()))
        })?;
        if within(&real_dir, &self.write_roots) {
            Ok(real_dir.join(name))
        } else {
            Err(forbidden(path, "write"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("opts_paths_{name}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).expect("temp dir");
        p
    }

    #[test]
    fn reads_and_writes_stay_inside_their_roots() {
        let (read, write, other) = (temp("r"), temp("w"), temp("o"));
        std::fs::write(read.join("a.cfg"), "x").expect("file");
        std::fs::write(other.join("b.cfg"), "x").expect("file");
        let policy = PathPolicy {
            read_roots: vec![read.clone()],
            write_roots: vec![write.clone()],
        };
        assert!(policy.check_read(&read.join("a.cfg")).is_ok());
        assert_eq!(
            policy.check_read(&other.join("b.cfg")).unwrap_err().code,
            code::FORBIDDEN_PATH
        );
        assert_eq!(
            policy.check_read(&read.join("missing")).unwrap_err().code,
            code::NOT_FOUND
        );
        assert!(policy.check_write(&write.join("shot.png")).is_ok());
        assert_eq!(
            policy
                .check_write(&other.join("shot.png"))
                .unwrap_err()
                .code,
            code::FORBIDDEN_PATH
        );
    }

    #[test]
    fn dot_dot_segments_cannot_escape() {
        let (read, other) = (temp("r2"), temp("o2"));
        std::fs::write(other.join("secret"), "x").expect("file");
        let policy = PathPolicy {
            read_roots: vec![read.clone()],
            write_roots: vec![read.clone()],
        };
        let sneaky = read
            .join("..")
            .join(other.file_name().expect("name"))
            .join("secret");
        assert_eq!(
            policy.check_read(&sneaky).unwrap_err().code,
            code::FORBIDDEN_PATH
        );
        let sneaky_write = read.join("..").join("evil.png");
        assert_eq!(
            policy.check_write(&sneaky_write).unwrap_err().code,
            code::FORBIDDEN_PATH
        );
    }

    #[cfg(unix)]
    #[test]
    fn symlinks_are_resolved_before_the_check() {
        let (read, other) = (temp("r3"), temp("o3"));
        std::fs::write(other.join("secret"), "x").expect("file");
        std::os::unix::fs::symlink(&other, read.join("link")).expect("symlink");
        let policy = PathPolicy {
            read_roots: vec![read.clone()],
            write_roots: vec![],
        };
        assert_eq!(
            policy
                .check_read(&read.join("link").join("secret"))
                .unwrap_err()
                .code,
            code::FORBIDDEN_PATH
        );
    }
}
