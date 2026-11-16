//! Records produced by the filesystem walk.

/// A directory found during the walk.
#[derive(Debug, Clone)]
pub struct DirRec {
    /// Path relative to the OMSI root, forward slashes (`""` for the root).
    pub rel_path: String,
    /// Parent's relative path (`None` for the root).
    pub parent: Option<String>,
    /// Base name.
    pub name: String,
    /// Depth below the root (root = 0).
    pub depth: i64,
    /// Modification time, seconds since the Unix epoch.
    pub mtime: i64,
    /// Number of immediate sub-directories.
    pub child_dirs: i64,
    /// Number of immediate files (including symlinks).
    pub child_files: i64,
}

/// A file (or symlink) found during the walk.
#[derive(Debug, Clone)]
pub struct FileRec {
    /// Relative path of the enclosing directory.
    pub dir: String,
    /// Path relative to the OMSI root, forward slashes.
    pub rel_path: String,
    /// Base name.
    pub name: String,
    /// Lowercase extension without the dot.
    pub ext: String,
    /// Size in bytes.
    pub size: i64,
    /// Modification time, seconds since the Unix epoch.
    pub mtime: i64,
    /// Whether the entry is a symbolic link.
    pub is_symlink: bool,
    /// Link target, if a symlink.
    pub symlink_target: Option<String>,
    /// Whether the extension marks a backup (`.bu`, `.bak`, `.aod`, `.old`).
    pub is_backup: bool,
}

/// Result of a walk.
#[derive(Debug, Default)]
pub struct ScanOut {
    /// All directories, parents before children after [`ScanOut::sort`].
    pub dirs: Vec<DirRec>,
    /// All files.
    pub files: Vec<FileRec>,
}

impl ScanOut {
    pub(super) fn merge(&mut self, other: ScanOut) {
        self.dirs.extend(other.dirs);
        self.files.extend(other.files);
    }

    /// Orders directories so that parents precede children.
    pub fn sort(&mut self) {
        self.dirs.sort_by(|a, b| {
            a.depth
                .cmp(&b.depth)
                .then_with(|| a.rel_path.cmp(&b.rel_path))
        });
    }
}
