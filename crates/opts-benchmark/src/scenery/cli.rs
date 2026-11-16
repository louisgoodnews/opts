//! Command-line arguments of `opts-benchmark scenery …` and their execution.

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result};
use clap::Subcommand;

use super::diag::EventSink;
use super::supervisor::{self, RunManifest, RunOptions};
use super::verdict::Thresholds;
use super::worker::{self, WorkerConfig};
use super::{corpus, lookups, preflight, report};

mod args;
pub use args::{PreflightArgs, ReportArgs, RunArgs, WorkerArgs};

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
    /// CPU-only survey: tiers, context classes and tags of the corpus (no rendering).
    Preflight(PreflightArgs),
}

/// Runs the preflight survey; returns the class counts as text.
///
/// # Errors
/// Corpus or I/O failures.
fn survey(args: &PreflightArgs) -> Result<String> {
    use std::collections::BTreeMap;
    use std::io::Write as _;
    let sel = corpus::Selection {
        tier: args.tier.clone(),
        filter: args.filter.clone(),
        ..corpus::Selection::default()
    };
    let entries = corpus::select(corpus::entries(&args.omsi_path)?, &sel)?;
    let catalogue = lookups::open(args.catalogue.as_deref());
    let install = lookups::install(&args.omsi_path, catalogue.as_ref());
    let lines = preflight::survey(&install, &entries);
    let mut file = std::io::BufWriter::new(std::fs::File::create(&args.out)?);
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for l in &lines {
        writeln!(file, "{}", serde_json::to_string(l)?)?;
        for c in l.class.iter().chain(&l.categories) {
            *counts.entry(c.clone()).or_default() += 1;
        }
        *counts.entry(format!("tier:{}", l.tier)).or_default() += 1;
    }
    let mut text = format!("{} objects\n", lines.len());
    for (k, n) in counts {
        text.push_str(&format!("{k}: {n}\n"));
    }
    Ok(text)
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
            scripts: self.scripts,
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
        scripts: o.scripts,
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
        SceneryCommand::Preflight(args) => survey(&args),
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
