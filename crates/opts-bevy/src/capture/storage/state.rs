//! The captures state: the captures directory, the session id, the name counter and the free space.

use std::path::{Path, PathBuf};

use bevy::prelude::Resource;

use super::super::error::CaptureError;

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
