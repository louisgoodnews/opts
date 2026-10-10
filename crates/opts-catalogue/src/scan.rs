//! Parallel filesystem walk producing directory and file records.

use std::fs;
use std::path::Path;
use std::time::UNIX_EPOCH;

use rayon::prelude::*;

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
    fn merge(&mut self, other: ScanOut) {
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

fn mtime_of(md: &fs::Metadata) -> i64 {
    md.modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(0))
}

fn join(parent: &str, name: &str) -> String {
    if parent.is_empty() {
        name.to_string()
    } else {
        format!("{parent}/{name}")
    }
}

/// Walks one directory (recursively when `recurse`), appending records to `out`.
/// Sub-directories are returned instead of recursed into when `defer` is set.
fn walk(
    abs: &Path,
    rel: &str,
    parent: Option<&str>,
    depth: i64,
    recurse: bool,
    out: &mut ScanOut,
    deferred: &mut Vec<(std::path::PathBuf, String, i64)>,
) {
    let Ok(rd) = fs::read_dir(abs) else { return };
    let mtime = fs::metadata(abs).map(|m| mtime_of(&m)).unwrap_or(0);
    let (mut child_dirs, mut child_files) = (0i64, 0i64);
    let mut subdirs = Vec::new();

    for entry in rd.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let Ok(ft) = entry.file_type() else { continue };
        let rel_child = join(rel, &name);
        if ft.is_dir() {
            child_dirs += 1;
            subdirs.push((entry.path(), rel_child, name));
        } else {
            child_files += 1;
            let path = entry.path();
            let is_symlink = ft.is_symlink();
            let md = if is_symlink {
                fs::symlink_metadata(&path)
            } else {
                entry.metadata()
            };
            let Ok(md) = md else { continue };
            let ext = name
                .rsplit_once('.')
                .map(|(_, e)| e.to_ascii_lowercase())
                .unwrap_or_default();
            out.files.push(FileRec {
                dir: rel.to_string(),
                rel_path: rel_child,
                is_backup: matches!(ext.as_str(), "bu" | "bak" | "aod" | "old"),
                ext,
                name,
                size: i64::try_from(md.len()).unwrap_or(0),
                mtime: mtime_of(&md),
                is_symlink,
                symlink_target: is_symlink
                    .then(|| {
                        fs::read_link(&path)
                            .ok()
                            .map(|t| t.to_string_lossy().into_owned())
                    })
                    .flatten(),
            });
        }
    }

    out.dirs.push(DirRec {
        rel_path: rel.to_string(),
        parent: parent.map(str::to_string),
        name: rel.rsplit('/').next().unwrap_or("").to_string(),
        depth,
        mtime,
        child_dirs,
        child_files,
    });

    for (path, rel_child, _) in subdirs {
        if recurse {
            walk(&path, &rel_child, Some(rel), depth + 1, true, out, deferred);
        } else {
            deferred.push((path, rel_child, depth + 1));
        }
    }
}

/// Scans the whole installation. The first two directory levels are walked sequentially,
/// everything below runs in parallel (one rayon task per depth-2 directory).
#[must_use]
pub fn scan_installation(root: &Path) -> ScanOut {
    let mut out = ScanOut::default();
    let mut level1 = Vec::new();
    walk(root, "", None, 0, false, &mut out, &mut level1);

    let mut level2 = Vec::new();
    for (path, rel, depth) in level1 {
        let parent = rel.rsplit_once('/').map_or("", |(p, _)| p).to_string();
        walk(
            &path,
            &rel,
            Some(&parent),
            depth,
            false,
            &mut out,
            &mut level2,
        );
    }

    let parts: Vec<ScanOut> = level2
        .par_iter()
        .map(|(path, rel, depth)| {
            let mut o = ScanOut::default();
            let parent = rel.rsplit_once('/').map_or("", |(p, _)| p).to_string();
            let mut none = Vec::new();
            walk(path, rel, Some(&parent), *depth, true, &mut o, &mut none);
            o
        })
        .collect();
    for p in parts {
        out.merge(p);
    }
    out.sort();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scans_nested_tree_with_depths_and_extensions() {
        let dir = std::env::temp_dir().join(format!("opts_cat_scan_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("Vehicles/Bus1/model")).unwrap();
        fs::write(dir.join("Vehicles/Bus1/Bus1.BUS"), b"x").unwrap();
        fs::write(dir.join("Vehicles/Bus1/model/a.o3d.bak"), b"yy").unwrap();
        let out = scan_installation(&dir);
        assert!(out
            .dirs
            .iter()
            .any(|d| d.rel_path == "Vehicles/Bus1/model" && d.depth == 3));
        let bus = out.files.iter().find(|f| f.name == "Bus1.BUS").unwrap();
        assert_eq!(
            (bus.ext.as_str(), bus.size, bus.is_backup),
            ("bus", 1, false)
        );
        assert!(out.files.iter().any(|f| f.is_backup));
        assert_eq!(out.dirs[0].rel_path, "", "root first");
        let _ = fs::remove_dir_all(&dir);
    }
}
