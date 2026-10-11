//! Command-line arguments of `opts-benchmark scenery …` and their execution.

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result};
use clap::{Args, Subcommand};

use super::diag::EventSink;
use super::supervisor::{self, RunManifest, RunOptions};
use super::verdict::Thresholds;
use super::worker::{self, WorkerConfig};
use super::{mapgen, report};

/// `opts-benchmark scenery` subcommands.
#[derive(Debug, Subcommand)]
// Parsed once per process; boxing the large variant buys nothing.
#[allow(clippy::large_enum_variant)]
pub enum SceneryCommand {
    /// Verify scenery objects (supervisor).
    Run(RunArgs),
    /// Worker process (started by `run`).
    Worker(WorkerArgs),
    /// Rewrite the summary files of a run, or compare two runs.
    Report(ReportArgs),
}

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
    /// Run the script-effect check (Phase C; reports `S-NOT-RUN` until scenery scripts run).
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

/// The game's catalogue database, if it exists and describes `root`.
fn default_catalogue(root: &Path) -> Option<PathBuf> {
    let config = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    let path = config.join("opts/catalogue.db");
    let cat = opts_catalogue::Catalogue::open_read_only(&path).ok()?;
    let same = cat.root().ok().flatten().is_some_and(|r| r == root);
    if !same {
        tracing::warn!(path = %path.display(), "catalogue describes another OMSI root; not used");
    }
    same.then_some(path)
}

impl RunArgs {
    /// Converts the arguments into run options.
    #[must_use]
    pub fn options(&self) -> RunOptions {
        let catalogue = if self.no_catalogue {
            None
        } else {
            self.catalogue
                .clone()
                .or_else(|| default_catalogue(&self.omsi_path))
        };
        if self.scripts {
            tracing::warn!(
                "--scripts: scenery scripts do not run yet; scripted objects report S-NOT-RUN"
            );
        }
        RunOptions {
            omsi_root: self.omsi_path.clone(),
            out: self.out.clone().unwrap_or_else(supervisor::default_out),
            resume: self.resume.clone(),
            workers: self.workers,
            tier: self.tier.clone(),
            filter: self.filter.clone(),
            list: self.list.clone(),
            sample: self.sample,
            limit: self.limit,
            keep_images: self.keep_images.clone(),
            anchor: self.anchor.clone(),
            timeout_s: self.timeout_s,
            recycle: self.recycle,
            catalogue,
            cache_dir: self.cache_dir.clone(),
            seed_fault: self.seed_fault.clone(),
            thresholds: Thresholds::default(),
        }
    }
}

/// Builds a worker's configuration from its run directory.
///
/// # Errors
/// `run.json` is missing or invalid.
pub fn worker_config(args: &WorkerArgs) -> Result<WorkerConfig> {
    let text = std::fs::read_to_string(args.run_dir.join("run.json")).context("run.json")?;
    let m: RunManifest = serde_json::from_str(&text)?;
    let o = m.options;
    Ok(WorkerConfig {
        run_dir: args.run_dir.clone(),
        id: args.worker_id,
        omsi_root: o.omsi_root,
        catalogue: o.catalogue,
        anchor: o.anchor,
        max_objects: o.recycle.max(1),
        object_timeout: Duration::from_secs(o.timeout_s),
        keep_images: o.keep_images,
        seed_fault: o.seed_fault,
        thresholds: o.thresholds,
    })
}

/// Executes a `scenery` subcommand; returns the text the binary prints.
///
/// # Errors
/// Whatever the subcommand fails with.
pub fn execute(cmd: SceneryCommand, sink: EventSink) -> Result<String> {
    match cmd {
        SceneryCommand::Run(args) => {
            let dir = supervisor::run(args.options())?;
            let records = report::latest(report::read_results(&dir.join("results.jsonl")));
            let counts = supervisor::verdict_counts(&records)
                .iter()
                .map(|(v, n)| format!("{v:?} {n}"))
                .collect::<Vec<_>>()
                .join(", ");
            Ok(format!("run directory: {}\n{counts}", dir.display()))
        }
        SceneryCommand::Worker(args) => {
            worker::run(worker_config(&args)?, sink)?;
            Ok(String::new())
        }
        SceneryCommand::Report(args) => {
            let records = report::latest(report::read_results(&args.run_dir.join("results.jsonl")));
            report::write_summaries(&args.run_dir, &records)?;
            match args.compare {
                Some(other) => {
                    let old = report::latest(report::read_results(&other.join("results.jsonl")));
                    Ok(report::compare(&old, &records))
                }
                None => Ok(format!("{} records summarised", records.len())),
            }
        }
    }
}
