//! The supervisor: prepares the run directory, hands objects to worker processes one at a time,
//! restarts workers that crash, hang or recycle themselves, and writes the report.
//!
//! Objects are dispatched dynamically over each worker's stdin rather than as fixed shards, so a
//! slow worker does not hold back the others and a restarted worker simply asks for the next
//! object. A crashed or hung object is recorded as `F-CRASH` / `F-TIMEOUT` and not retried.

use std::collections::{HashSet, VecDeque};
use std::io::{BufRead, BufReader, Write as _};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender};
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};

use super::corpus::{self, Selection};
use super::protocol::WorkerMsg;
use super::report::{self, ResultsWriter};
use super::{ObjectEntry, Record, Verdict};

mod options;
pub use options::{RunManifest, RunOptions};

mod pool;
use pool::Pool;

fn commit() -> String {
    Command::new("git")
        .args([
            "-C",
            env!("CARGO_MANIFEST_DIR"),
            "rev-parse",
            "--short",
            "HEAD",
        ])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown".into())
}

/// Default parent of run directories: `$XDG_DATA_HOME/opts/benchmark` (or `~/.local/share/…`).
#[must_use]
pub fn default_out() -> PathBuf {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
        .unwrap_or_else(|| PathBuf::from("."))
        .join("opts/benchmark")
}

/// Runs the whole benchmark; returns the run directory.
///
/// # Errors
/// Corpus or I/O failures, or workers that cannot start.
pub fn run(opts: RunOptions) -> Result<PathBuf> {
    let (run_dir, manifest) = prepare(&opts)?;
    let objects = corpus::read_jsonl(&run_dir.join("objects.jsonl"))?;
    let done: HashSet<String> = report::read_results(&run_dir.join("results.jsonl"))
        .into_iter()
        .map(|r| r.key)
        .collect();
    let pending: VecDeque<ObjectEntry> = objects
        .into_iter()
        .filter(|e| !done.contains(&e.key))
        .collect();
    tracing::info!(run = %run_dir.display(), pending = pending.len(), done = done.len(), "scenery benchmark starting");
    let mut pool = Pool::new(&run_dir, &opts)?;
    pool.dispatch(pending)?;
    pool.shutdown();
    let records = report::latest(report::read_results(&run_dir.join("results.jsonl")));
    let have: HashSet<&str> = records.iter().map(|r| r.key.as_str()).collect();
    let missing = corpus::read_jsonl(&run_dir.join("objects.jsonl"))?
        .iter()
        .filter(|e| !have.contains(e.key.as_str()))
        .count();
    if missing > 0 {
        tracing::error!(
            missing,
            "objects without a record; resume the run to retry them"
        );
    }
    report::write_summaries(&run_dir, &records)?;
    let mut manifest = manifest;
    manifest.finished = Some(chrono::Utc::now().to_rfc3339());
    std::fs::write(
        run_dir.join("run.json"),
        serde_json::to_string_pretty(&manifest)?,
    )?;
    Ok(run_dir)
}

/// Creates (or reopens) the run directory, `objects.jsonl` and `run.json`.
fn prepare(opts: &RunOptions) -> Result<(PathBuf, RunManifest)> {
    if let Some(id) = &opts.resume {
        let dir = opts.out.join(id);
        let text =
            std::fs::read_to_string(dir.join("run.json")).context("run.json of the resumed run")?;
        let manifest: RunManifest = serde_json::from_str(&text)?;
        return Ok((dir, manifest));
    }
    let commit = commit();
    let run_id = format!("{}-{commit}", chrono::Utc::now().format("%Y%m%dT%H%M%SZ"));
    let dir = opts.out.join(&run_id);
    std::fs::create_dir_all(&dir)?;
    let list = match &opts.list {
        Some(p) => Some(
            std::fs::read_to_string(p)?
                .lines()
                .map(str::trim)
                .filter(|l| !l.is_empty())
                .map(String::from)
                .collect(),
        ),
        None => None,
    };
    let sel = Selection {
        tier: opts.tier.clone(),
        filter: opts.filter.clone(),
        list,
        sample: opts.sample,
        limit: opts.limit,
    };
    let entries = corpus::select(corpus::entries(&opts.omsi_root)?, &sel)?;
    corpus::write_jsonl(&dir.join("objects.jsonl"), &entries)?;
    let manifest = RunManifest {
        run_id,
        commit,
        started: chrono::Utc::now().to_rfc3339(),
        finished: None,
        objects: entries.len(),
        options: opts.clone(),
    };
    std::fs::write(
        dir.join("run.json"),
        serde_json::to_string_pretty(&manifest)?,
    )?;
    Ok((dir, manifest))
}

/// Counts verdicts, for the command-line summary.
#[must_use]
pub fn verdict_counts(records: &[Record]) -> [(Verdict, usize); 4] {
    [
        Verdict::Pass,
        Verdict::Fail,
        Verdict::Unknown,
        Verdict::Skipped,
    ]
    .map(|v| (v, records.iter().filter(|r| r.verdict == v).count()))
}
