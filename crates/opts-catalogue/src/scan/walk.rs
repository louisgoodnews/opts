//! Recursive directory walk producing the records of one subtree.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use super::records::{DirRec, FileRec, ScanOut};

fn mtime_of(md: &fs::Metadata) -> i64 {
    md.modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(0))
}

pub(super) fn join(parent: &str, name: &str) -> String {
    if parent.is_empty() {
        name.to_string()
    } else {
        format!("{parent}/{name}")
    }
}

/// Walks one directory (recursively when `recurse`), appending records to `out`.
/// Sub-directories are returned instead of recursed into when `defer` is set.
pub(super) fn walk(
    abs: &Path,
    rel: &str,
    parent: Option<&str>,
    depth: i64,
    recurse: bool,
    out: &mut ScanOut,
    deferred: &mut Vec<(PathBuf, String, i64)>,
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
