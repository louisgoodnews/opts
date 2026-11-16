//! Options of a run and the manifest it writes (`run.json`).

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::super::verdict::Thresholds;

/// Options of `opts-benchmark scenery run`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunOptions {
    /// OMSI installation root (read only).
    pub omsi_root: PathBuf,
    /// Parent folder of run directories.
    pub out: PathBuf,
    /// Run id to resume, if any.
    pub resume: Option<String>,
    /// Number of worker processes.
    pub workers: usize,
    /// Tier filter (`referenced` or `all`).
    pub tier: String,
    /// Glob filter.
    pub filter: Option<String>,
    /// File with relative `.sco` paths, one per line.
    pub list: Option<PathBuf>,
    /// Spread sample size.
    pub sample: Option<usize>,
    /// Maximum number of objects.
    pub limit: Option<usize>,
    /// Image keep mode (`none`, `fail`, `all`).
    pub keep_images: String,
    /// Entry anchor `.sco`.
    pub anchor: String,
    /// Object time budget in seconds.
    pub timeout_s: u64,
    /// Objects per worker before it is replaced.
    pub recycle: usize,
    /// Catalogue database (read-only).
    pub catalogue: Option<PathBuf>,
    /// `XDG_CACHE_HOME` for the workers (decode cache); inherited when `None`.
    pub cache_dir: Option<PathBuf>,
    /// Run the script-effect check on scripted objects.
    #[serde(default)]
    pub scripts: bool,
    /// Seeded fault for the acceptance tests.
    pub seed_fault: Option<String>,
    /// Decision thresholds.
    pub thresholds: Thresholds,
}

/// Contents of `run.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunManifest {
    /// Run id (UTC timestamp and commit).
    pub run_id: String,
    /// Commit of the benchmark build.
    pub commit: String,
    /// Start time (UTC, RFC 3339).
    pub started: String,
    /// End time, once finished.
    pub finished: Option<String>,
    /// Objects selected.
    pub objects: usize,
    /// Options.
    pub options: RunOptions,
}
