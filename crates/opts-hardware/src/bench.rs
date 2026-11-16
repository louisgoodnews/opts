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

mod workloads;

use workloads::*;

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
mod tests;
