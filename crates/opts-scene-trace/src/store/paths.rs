//! Where the trace files go: the directory, the current session and the path forms.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub(super) static DIRECTORY: Mutex<Option<PathBuf>> = Mutex::new(None);
pub(super) static SESSION: Mutex<Option<(String, PathBuf)>> = Mutex::new(None);

pub(crate) fn set_directory(dir: PathBuf) {
    if let Ok(mut d) = DIRECTORY.lock() {
        *d = Some(dir);
    }
}

pub(super) fn directory() -> PathBuf {
    DIRECTORY
        .lock()
        .ok()
        .and_then(|d| d.clone())
        .unwrap_or_else(|| std::env::temp_dir().join("opts-trace"))
}

pub(crate) fn remember_session(map: &str, root: &Path) {
    if let Ok(mut s) = SESSION.lock() {
        *s = Some((map.to_owned(), root.to_path_buf()));
    }
}

pub(crate) fn norm(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/").to_lowercase()
}

pub(crate) fn file_name_of(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}
