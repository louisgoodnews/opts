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
use serde::{Deserialize, Serialize};

use super::corpus::{self, Selection};
use super::protocol::WorkerMsg;
use super::report::{self, ResultsWriter};
use super::verdict::Thresholds;
use super::{ObjectEntry, Record, Verdict};

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

enum Event {
    Msg(WorkerMsg),
    Eof,
}

enum State {
    Starting(Instant),
    Idle,
    Busy(ObjectEntry, Instant),
    Leaving,
}

struct Slot {
    child: Child,
    stdin: Option<ChildStdin>,
    state: State,
    generation: u64,
    failed_starts: usize,
}

struct Pool {
    run_dir: PathBuf,
    opts: RunOptions,
    slots: Vec<Slot>,
    tx: Sender<(usize, u64, Event)>,
    rx: Receiver<(usize, u64, Event)>,
    results: ResultsWriter,
    finished: usize,
    total: usize,
    started: Instant,
}

impl Pool {
    fn new(run_dir: &Path, opts: &RunOptions) -> Result<Self> {
        let (tx, rx) = channel();
        let mut pool = Self {
            run_dir: run_dir.to_path_buf(),
            opts: opts.clone(),
            slots: Vec::new(),
            tx,
            rx,
            results: ResultsWriter::open(&run_dir.join("results.jsonl"))?,
            finished: 0,
            total: 0,
            started: Instant::now(),
        };
        for i in 0..opts.workers.max(1) {
            let slot = pool.spawn(i, 0)?;
            pool.slots.push(slot);
        }
        Ok(pool)
    }

    fn worker_dir(&self, i: usize) -> PathBuf {
        self.run_dir.join(format!("work/worker-{i}"))
    }

    fn spawn(&self, i: usize, generation: u64) -> Result<Slot> {
        let dir = self.worker_dir(i);
        std::fs::create_dir_all(dir.join("config"))?;
        std::fs::create_dir_all(dir.join("data"))?;
        let log = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(dir.join("worker.log"))?;
        let mut cmd = Command::new(std::env::current_exe()?);
        cmd.args(["scenery", "worker", "--run-dir"])
            .arg(&self.run_dir)
            .args(["--worker-id", &i.to_string()])
            .env("XDG_CONFIG_HOME", dir.join("config"))
            .env("XDG_DATA_HOME", dir.join("data"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::from(log));
        if let Some(c) = &self.opts.cache_dir {
            cmd.env("XDG_CACHE_HOME", c);
        }
        let mut child = cmd.spawn().context("start worker")?;
        let stdout = child.stdout.take().context("worker stdout")?;
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                if let Some(m) = WorkerMsg::from_line(&line) {
                    if tx.send((i, generation, Event::Msg(m))).is_err() {
                        return;
                    }
                }
            }
            let _ = tx.send((i, generation, Event::Eof));
        });
        Ok(Slot {
            stdin: child.stdin.take(),
            child,
            state: State::Starting(Instant::now()),
            generation,
            failed_starts: 0,
        })
    }

    fn dispatch(&mut self, mut pending: VecDeque<ObjectEntry>) -> Result<()> {
        self.total = pending.len();
        let timeout = Duration::from_secs(self.opts.timeout_s) + Duration::from_secs(15);
        loop {
            for i in 0..self.slots.len() {
                if matches!(self.slots[i].state, State::Idle) {
                    if let Some(entry) = pending.pop_front() {
                        self.send(i, entry);
                    }
                }
            }
            let busy = self
                .slots
                .iter()
                .any(|s| matches!(s.state, State::Busy(..) | State::Starting(_)));
            if pending.is_empty() && !busy {
                return Ok(());
            }
            match self.rx.recv_timeout(Duration::from_millis(200)) {
                Ok((i, generation, ev)) if self.slots[i].generation == generation => {
                    self.handle(i, ev)?
                }
                Ok(_) | Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => bail!("worker channel closed"),
            }
            self.check_timeouts(timeout)?;
        }
    }

    fn send(&mut self, i: usize, entry: ObjectEntry) {
        let line = serde_json::to_string(&entry).unwrap_or_default();
        let slot = &mut self.slots[i];
        let ok = slot
            .stdin
            .as_mut()
            .is_some_and(|s| writeln!(s, "{line}").and_then(|()| s.flush()).is_ok());
        // A failed write means the worker died; its EOF event records the crash.
        slot.state = State::Busy(entry, Instant::now());
        if !ok {
            tracing::warn!(worker = i, "cannot write to worker");
        }
    }

    fn handle(&mut self, i: usize, ev: Event) -> Result<()> {
        match ev {
            Event::Msg(WorkerMsg::Ready) => {
                self.slots[i].state = State::Idle;
                self.slots[i].failed_starts = 0;
            }
            Event::Msg(WorkerMsg::Start { .. }) => {}
            Event::Msg(WorkerMsg::Result { record }) => {
                self.record(&record)?;
                if matches!(self.slots[i].state, State::Busy(..)) {
                    self.slots[i].state = State::Idle;
                }
            }
            Event::Msg(WorkerMsg::Recycle { reason }) => {
                tracing::info!(worker = i, %reason, "worker recycles");
                self.slots[i].state = State::Leaving;
            }
            Event::Eof => self.lost(i, "F-CRASH")?,
        }
        Ok(())
    }

    fn record(&mut self, r: &Record) -> Result<()> {
        self.results.append(r)?;
        self.finished += 1;
        if self.finished.is_multiple_of(25) || self.finished == self.total {
            let rate = self.started.elapsed().as_secs_f64() / self.finished as f64;
            tracing::info!(
                done = self.finished,
                total = self.total,
                s_per_object = format!("{rate:.3}"),
                "progress"
            );
        }
        Ok(())
    }

    /// A worker is gone (exit, crash or kill): record its object, start a successor.
    fn lost(&mut self, i: usize, category: &str) -> Result<()> {
        let _ = self.slots[i].child.kill();
        let _ = self.slots[i].child.wait();
        let state = std::mem::replace(&mut self.slots[i].state, State::Leaving);
        let failed_starts = match state {
            State::Busy(entry, since) => {
                tracing::warn!(worker = i, sco = %entry.sco, category, "object lost");
                let mut r =
                    Record::aborted(&entry, category, i, since.elapsed().as_millis() as u64);
                let dest = self.run_dir.join("repro").join(&entry.key);
                let map = self.worker_dir(i).join("map");
                if copy_dir(&map, &dest).is_ok() {
                    r.repro = Some(format!("XDG_CONFIG_HOME=$(mktemp -d) opts-game --autostart --map-path {} --omsi-path {} --camera free", dest.display(), self.opts.omsi_root.display()));
                }
                self.record(&r)?;
                0
            }
            State::Starting(_) => self.slots[i].failed_starts + 1,
            _ => 0,
        };
        if failed_starts >= 3 {
            bail!(
                "worker {i} failed to start three times; see {}",
                self.worker_dir(i).join("worker.log").display()
            );
        }
        let generation = self.slots[i].generation + 1;
        let mut slot = self.spawn(i, generation)?;
        slot.failed_starts = failed_starts;
        self.slots[i] = slot;
        Ok(())
    }

    fn check_timeouts(&mut self, object_timeout: Duration) -> Result<()> {
        for i in 0..self.slots.len() {
            let expired = match &self.slots[i].state {
                State::Busy(_, since) => since.elapsed() > object_timeout,
                State::Starting(since) => since.elapsed() > Duration::from_secs(180),
                _ => false,
            };
            if expired {
                let cat = if matches!(self.slots[i].state, State::Busy(..)) {
                    "F-TIMEOUT"
                } else {
                    "F-CRASH"
                };
                self.lost(i, cat)?;
            }
        }
        Ok(())
    }

    fn shutdown(&mut self) {
        for s in &mut self.slots {
            s.stdin = None;
        }
        for s in &mut self.slots {
            let deadline = Instant::now() + Duration::from_secs(10);
            while Instant::now() < deadline {
                if matches!(s.child.try_wait(), Ok(Some(_))) {
                    break;
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            let _ = s.child.kill();
            let _ = s.child.wait();
        }
    }
}

fn copy_dir(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for e in std::fs::read_dir(from)?.flatten() {
        std::fs::copy(e.path(), to.join(e.file_name()))?;
    }
    Ok(())
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
