//! Arguments of the `scenery` subcommands (`run`, `worker`, `report`, `preflight`).

use std::path::PathBuf;

use clap::Args;

use super::super::mapgen;

/// Arguments of `scenery run`.
#[derive(Debug, Args)]
pub struct RunArgs {
    /// OMSI 2 installation root (only read).
    #[arg(long)]
    pub omsi_path: PathBuf,
    /// Parent folder of run directories (default `$XDG_DATA_HOME/opts/benchmark`).
    #[arg(long)]
    pub out: Option<PathBuf>,
    /// Worker processes.
    #[arg(long, default_value_t = 3)]
    pub workers: usize,
    /// `referenced` (objects placed or attached by a map) or `all`.
    #[arg(long, default_value = "all")]
    pub tier: String,
    /// Case-insensitive glob on the relative `.sco` path.
    #[arg(long)]
    pub filter: Option<String>,
    /// File with relative `.sco` paths to verify, one per line.
    #[arg(long)]
    pub list: Option<PathBuf>,
    /// Spread sample of about this many objects (deterministic, by key order).
    #[arg(long)]
    pub sample: Option<usize>,
    /// At most this many objects.
    #[arg(long)]
    pub limit: Option<usize>,
    /// Run the script-effect check on scripted objects (Phase C; without it they report `not-checked`).
    #[arg(long)]
    pub scripts: bool,
    /// Images to keep: `none`, `fail` (non-pass) or `all`.
    #[arg(long, default_value = "fail")]
    pub keep_images: String,
    /// Resume the run with this id.
    #[arg(long)]
    pub resume: Option<String>,
    /// Entry anchor `.sco` (relative to the OMSI root).
    #[arg(long, default_value = mapgen::DEFAULT_ANCHOR)]
    pub anchor: String,
    /// Time budget per object in seconds.
    #[arg(long, default_value_t = 30)]
    pub timeout_s: u64,
    /// Objects per worker before it is replaced by a fresh process.
    #[arg(long, default_value_t = 2000)]
    pub recycle: usize,
    /// Catalogue database (default: the game's `catalogue.db`, opened read-only).
    #[arg(long)]
    pub catalogue: Option<PathBuf>,
    /// Resolve paths on disk only, without the catalogue.
    #[arg(long)]
    pub no_catalogue: bool,
    /// `XDG_CACHE_HOME` of the workers (decoder cache); inherited when not given.
    #[arg(long)]
    pub cache_dir: Option<PathBuf>,
    /// Deliberate fault: `heading-mirror`, `offset-10m` or `scale-2x` (acceptance tests).
    #[arg(long)]
    pub seed_fault: Option<String>,
}

/// Arguments of `scenery worker`.
#[derive(Debug, Args)]
pub struct WorkerArgs {
    /// Run directory (its `run.json` holds the options).
    #[arg(long)]
    pub run_dir: PathBuf,
    /// Worker number.
    #[arg(long)]
    pub worker_id: usize,
}

/// Arguments of `scenery report`.
#[derive(Debug, Args)]
pub struct ReportArgs {
    /// Run directory.
    #[arg(long)]
    pub run_dir: PathBuf,
    /// Another run directory to compare with (lists changed verdicts).
    #[arg(long)]
    pub compare: Option<PathBuf>,
}

/// Arguments of `scenery preflight`.
#[derive(Debug, Args)]
pub struct PreflightArgs {
    /// OMSI 2 installation root (only read).
    #[arg(long)]
    pub omsi_path: PathBuf,
    /// File to write one JSON line per object to.
    #[arg(long)]
    pub out: PathBuf,
    /// `referenced` or `all`.
    #[arg(long, default_value = "all")]
    pub tier: String,
    /// Case-insensitive glob on the relative `.sco` path.
    #[arg(long)]
    pub filter: Option<String>,
    /// Catalogue database; paths resolve on disk only when absent.
    #[arg(long)]
    pub catalogue: Option<PathBuf>,
}
