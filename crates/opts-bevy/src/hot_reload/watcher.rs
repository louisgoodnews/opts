//! Cross-platform file watching (`notify`), wrapped so that roots can be added and removed at run time.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};
use std::sync::Mutex;

use bevy::prelude::Resource;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use super::events::is_editor_scratch;

struct Inner {
    watcher: RecommendedWatcher,
    rx: Receiver<notify::Result<Event>>,
    roots: BTreeMap<PathBuf, bool>,
}

/// The file watcher of the running session.
#[derive(Resource)]
pub struct FileWatcher {
    inner: Mutex<Inner>,
}

/// Most directories that are watched at once (inotify has per-user limits).
pub const MAX_ROOTS: usize = 3000;

impl FileWatcher {
    /// Creates a watcher without roots.
    ///
    /// # Errors
    /// The `notify` error when the platform watcher cannot be created.
    pub fn new() -> notify::Result<Self> {
        let (tx, rx) = channel();
        let watcher = notify::recommended_watcher(move |event| {
            let _ = tx.send(event);
        })?;
        Ok(Self {
            inner: Mutex::new(Inner {
                watcher,
                rx,
                roots: BTreeMap::new(),
            }),
        })
    }

    /// Starts watching `path` (a directory, or a file's directory), recursively when asked.
    ///
    /// # Errors
    /// The `notify` error, or an error when [`MAX_ROOTS`] is reached.
    pub fn add(&self, path: &Path, recursive: bool) -> notify::Result<bool> {
        let Ok(mut inner) = self.inner.lock() else {
            return Ok(false);
        };
        if inner.roots.contains_key(path) {
            return Ok(false);
        }
        if inner.roots.len() >= MAX_ROOTS {
            return Err(notify::Error::generic("too many watched directories"));
        }
        let mode = if recursive {
            RecursiveMode::Recursive
        } else {
            RecursiveMode::NonRecursive
        };
        inner.watcher.watch(path, mode)?;
        inner.roots.insert(path.to_path_buf(), recursive);
        Ok(true)
    }

    /// Stops watching `path`; `false` when it was not watched.
    pub fn remove(&self, path: &Path) -> bool {
        let Ok(mut inner) = self.inner.lock() else {
            return false;
        };
        if inner.roots.remove(path).is_none() {
            return false;
        }
        let _ = inner.watcher.unwatch(path);
        true
    }

    /// The watched roots with their recursion flag.
    #[must_use]
    pub fn roots(&self) -> Vec<(PathBuf, bool)> {
        self.inner
            .lock()
            .map(|i| i.roots.iter().map(|(p, r)| (p.clone(), *r)).collect())
            .unwrap_or_default()
    }

    /// Whether `path` is already watched directly.
    #[must_use]
    pub fn is_watching(&self, path: &Path) -> bool {
        self.inner.lock().is_ok_and(|i| i.roots.contains_key(path))
    }

    /// Paths of files that were created or modified since the last call (no scratch files).
    pub fn poll(&self) -> Vec<PathBuf> {
        let Ok(inner) = self.inner.lock() else {
            return Vec::new();
        };
        let mut out = Vec::new();
        while let Ok(event) = inner.rx.try_recv() {
            let Ok(event) = event else { continue };
            if !matches!(event.kind, EventKind::Create(_) | EventKind::Modify(_)) {
                continue;
            }
            out.extend(
                event
                    .paths
                    .into_iter()
                    .filter(|p| !is_editor_scratch(p) && p.is_file()),
            );
        }
        out
    }
}
