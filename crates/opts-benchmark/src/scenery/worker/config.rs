//! Settings of one worker process.

use super::*;

/// Settings of one worker process.
#[derive(Debug, Clone)]
pub struct WorkerConfig {
    /// Run directory (images, repro folders, the worker's map folder).
    pub run_dir: PathBuf,
    /// Worker number.
    pub id: usize,
    /// OMSI installation root.
    pub omsi_root: PathBuf,
    /// SQLite catalogue, opened read-only.
    pub catalogue: Option<PathBuf>,
    /// Relative path of the entry anchor `.sco`.
    pub anchor: String,
    /// Exit after this many objects (the supervisor starts a fresh worker).
    pub max_objects: usize,
    /// Time budget of one object.
    pub object_timeout: Duration,
    /// Which images to keep: `none`, `fail` (non-pass) or `all`.
    pub keep_images: String,
    /// Run the script-effect check on scripted objects.
    pub scripts: bool,
    /// Deliberate fault for the seeded-fault tests (`heading-mirror`, `offset-10m`, `scale-2x`,
    /// `script-effect-off`).
    pub seed_fault: Option<String>,
    /// Decision thresholds.
    pub thresholds: super::super::verdict::Thresholds,
}
