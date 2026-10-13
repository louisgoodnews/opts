//! Optional micro-benchmarks: measured capability instead of guessing from model names.
//! Run only on request; visible progress; cancellable; std only (no platform code, no `unsafe`).
//! Units are arbitrary but comparable between runs on the same class of machine.

use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::fact::{Fact, Reason};
use crate::profile::HardwareProfile;

/// Shared flag: set from any thread to stop the run at the next checkpoint.
#[derive(Debug, Clone, Default)]
pub struct CancelToken(Arc<AtomicBool>);

impl CancelToken {
    /// A fresh, not cancelled token.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
    /// Requests cancellation.
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Relaxed);
    }
    /// Was cancellation requested?
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}

/// The measurement steps, in order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// One thread of integer work.
    CpuSingle,
    /// All threads of integer work.
    CpuMulti,
    /// Memory copy.
    Memory,
    /// Scratch-file read.
    Storage,
}

impl Step {
    /// Stable key.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::CpuSingle => "cpu_single",
            Self::CpuMulti => "cpu_multi",
            Self::Memory => "memory",
            Self::Storage => "storage",
        }
    }
    const ALL: [Self; 4] = [Self::CpuSingle, Self::CpuMulti, Self::Memory, Self::Storage];
}

/// What to measure and how long to spend.
#[derive(Debug, Clone)]
pub struct BenchOptions {
    /// Time per CPU step.
    pub cpu_time: Duration,
    /// Size of the memory-copy buffer, MiB.
    pub memory_mib: usize,
    /// Folder for the scratch file of the storage step (on the content drive); `None` skips it.
    pub storage_dir: Option<PathBuf>,
    /// Size of the scratch file, MiB.
    pub storage_mib: usize,
}

impl Default for BenchOptions {
    fn default() -> Self {
        Self {
            cpu_time: Duration::from_millis(250),
            memory_mib: 64,
            storage_dir: None,
            storage_mib: 64,
        }
    }
}

/// One thread of xorshift work until `until`; returns the number of operations.
fn spin(until: Instant, cancel: &CancelToken) -> u64 {
    let (mut x, mut ops) = (0x9E37_79B9_7F4A_7C15_u64, 0_u64);
    while Instant::now() < until && !cancel.is_cancelled() {
        for _ in 0..100_000 {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
        }
        ops += 100_000 * 3;
    }
    std::hint::black_box(x);
    ops
}

fn mops(ops: u64, elapsed: Duration) -> u32 {
    let s = elapsed.as_secs_f64().max(1e-6);
    u32::try_from((ops as f64 / s / 1e6) as u64).unwrap_or(u32::MAX)
}

fn cpu(threads: usize, time: Duration, cancel: &CancelToken) -> Fact<u32> {
    let start = Instant::now();
    let until = start + time;
    let ops: u64 = std::thread::scope(|s| {
        let handles: Vec<_> = (0..threads.max(1))
            .map(|_| s.spawn(|| spin(until, cancel)))
            .collect();
        handles.into_iter().map(|h| h.join().unwrap_or(0)).sum()
    });
    if cancel.is_cancelled() {
        Fact::Unknown(Reason::Cancelled)
    } else {
        Fact::Known(mops(ops, start.elapsed()))
    }
}

fn memory(mib: usize, cancel: &CancelToken) -> Fact<u32> {
    let len = mib.max(1) * 1024 * 1024;
    let src = vec![1_u8; len];
    let mut dst = vec![0_u8; len];
    let start = Instant::now();
    let mut copied = 0_usize;
    while start.elapsed() < Duration::from_millis(200) && !cancel.is_cancelled() {
        dst.copy_from_slice(&src);
        std::hint::black_box(&dst);
        copied += len;
    }
    if cancel.is_cancelled() {
        return Fact::Unknown(Reason::Cancelled);
    }
    let rate = copied as f64 / 1_048_576.0 / start.elapsed().as_secs_f64().max(1e-6);
    Fact::Known(u32::try_from(rate as u64).unwrap_or(u32::MAX))
}

/// Writes then reads back a scratch file (deleted afterwards). The read may be served by the OS
/// cache, so the figure is an upper bound of the drive; it still separates SSD from HDD classes
/// when the file exceeds the cache.
fn storage(dir: &std::path::Path, mib: usize, cancel: &CancelToken) -> Fact<u32> {
    let file = dir.join(format!("opts-hw-bench-{}.tmp", std::process::id()));
    let result = (|| -> Result<Fact<u32>, std::io::Error> {
        fs::create_dir_all(dir)?;
        let chunk = vec![0xA5_u8; 1024 * 1024];
        let mut f = fs::File::create(&file)?;
        for _ in 0..mib.max(1) {
            if cancel.is_cancelled() {
                return Ok(Fact::Unknown(Reason::Cancelled));
            }
            f.write_all(&chunk)?;
        }
        f.sync_all()?;
        drop(f);
        let mut f = fs::File::open(&file)?;
        let mut buf = vec![0_u8; 1024 * 1024];
        let (start, mut total) = (Instant::now(), 0_usize);
        loop {
            if cancel.is_cancelled() {
                return Ok(Fact::Unknown(Reason::Cancelled));
            }
            let n = f.read(&mut buf)?;
            if n == 0 {
                break;
            }
            total += n;
        }
        let rate = total as f64 / 1_048_576.0 / start.elapsed().as_secs_f64().max(1e-6);
        Ok(Fact::Known(u32::try_from(rate as u64).unwrap_or(u32::MAX)))
    })();
    let _ = fs::remove_file(&file);
    result.unwrap_or(Fact::Unknown(Reason::Unreadable))
}

/// Runs the benchmarks and stores the results (with the class hash) in `profile.bench`.
/// `progress(step, index, total)` is called before each step. A cancelled run keeps the steps
/// finished before and marks the rest `Unknown(cancelled)`.
pub fn run(
    profile: &mut HardwareProfile,
    opts: &BenchOptions,
    cancel: &CancelToken,
    progress: &mut dyn FnMut(Step, usize, usize),
) {
    let threads = profile.cpu.logical_cores.get().map_or(1, |c| c as usize);
    let total = Step::ALL.len();
    for (i, step) in Step::ALL.into_iter().enumerate() {
        progress(step, i, total);
        let b = &mut profile.bench;
        if cancel.is_cancelled() {
            match step {
                Step::CpuSingle => b.cpu_single_mops = Fact::Unknown(Reason::Cancelled),
                Step::CpuMulti => b.cpu_multi_mops = Fact::Unknown(Reason::Cancelled),
                Step::Memory => b.memory_mib_s = Fact::Unknown(Reason::Cancelled),
                Step::Storage => b.storage_read_mib_s = Fact::Unknown(Reason::Cancelled),
            }
            continue;
        }
        match step {
            Step::CpuSingle => b.cpu_single_mops = cpu(1, opts.cpu_time, cancel),
            Step::CpuMulti => b.cpu_multi_mops = cpu(threads, opts.cpu_time, cancel),
            Step::Memory => b.memory_mib_s = memory(opts.memory_mib, cancel),
            Step::Storage => {
                b.storage_read_mib_s = opts
                    .storage_dir
                    .as_deref()
                    .map_or(Fact::Unknown(Reason::NotRequested), |d| {
                        storage(d, opts.storage_mib, cancel)
                    });
            }
        }
    }
    if !cancel.is_cancelled() {
        profile.bench.measured_for_class = Fact::Known(profile.class_hash());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quick() -> BenchOptions {
        BenchOptions {
            cpu_time: Duration::from_millis(20),
            memory_mib: 4,
            storage_dir: None,
            storage_mib: 2,
        }
    }

    #[test]
    fn runs_and_reports_progress() {
        let mut p = HardwareProfile::default();
        let mut seen = Vec::new();
        run(&mut p, &quick(), &CancelToken::new(), &mut |s, i, n| {
            seen.push((s, i, n))
        });
        assert_eq!(seen.len(), 4);
        assert_eq!(seen[3], (Step::Storage, 3, 4));
        assert!(p.bench.cpu_single_mops.get().is_some_and(|v| v > 0));
        assert!(p.bench.cpu_multi_mops.get().is_some());
        assert!(p.bench.memory_mib_s.get().is_some_and(|v| v > 0));
        assert_eq!(
            p.bench.storage_read_mib_s,
            Fact::Unknown(Reason::NotRequested)
        );
        assert_eq!(p.bench.measured_for_class, Fact::Known(p.class_hash()));
    }

    #[test]
    fn storage_step_leaves_no_file_behind() {
        let dir = std::env::temp_dir().join(format!("opts-hw-bench-test-{}", std::process::id()));
        let mut p = HardwareProfile::default();
        let o = BenchOptions {
            storage_dir: Some(dir.clone()),
            ..quick()
        };
        run(&mut p, &o, &CancelToken::new(), &mut |_, _, _| {});
        assert!(p.bench.storage_read_mib_s.get().is_some());
        assert_eq!(fs::read_dir(&dir).map(Iterator::count).unwrap_or(0), 0);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn cancel_stops_the_run() {
        let c = CancelToken::new();
        let mut p = HardwareProfile::default();
        let c2 = c.clone();
        run(&mut p, &quick(), &c, &mut |s, _, _| {
            if s == Step::CpuMulti {
                c2.cancel();
            }
        });
        assert!(p.bench.cpu_single_mops.get().is_some());
        assert_eq!(p.bench.cpu_multi_mops, Fact::Unknown(Reason::Cancelled));
        assert_eq!(p.bench.memory_mib_s, Fact::Unknown(Reason::Cancelled));
        assert!(p.bench.measured_for_class.known().is_none());
    }
}
