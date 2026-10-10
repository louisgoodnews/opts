//! RAII guard ensuring non-blocking log writers are flushed on drop.

use tracing_appender::non_blocking::WorkerGuard;

/// RAII lifetime handle holding worker guards for all active non-blocking log appenders.
///
/// When this guard is dropped (e.g. during application exit or shutdown), the worker
/// thread input channels are closed and all pending buffer records are flushed to disk.
#[must_use = "LogGuard must be held for the application lifetime to ensure log flushing"]
pub struct LogGuard {
    guards: Vec<WorkerGuard>,
}

impl LogGuard {
    /// Creates a new `LogGuard` from an iterator of [`WorkerGuard`] instances.
    pub fn new(guards: Vec<WorkerGuard>) -> Self {
        Self { guards }
    }

    /// Appends an additional worker guard to this container.
    pub fn push(&mut self, guard: WorkerGuard) {
        self.guards.push(guard);
    }

    /// Number of active non-blocking worker threads held by this guard.
    pub fn len(&self) -> usize {
        self.guards.len()
    }

    /// Returns `true` if no worker guards are held.
    pub fn is_empty(&self) -> bool {
        self.guards.is_empty()
    }
}

impl std::fmt::Debug for LogGuard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LogGuard")
            .field("active_workers", &self.guards.len())
            .finish()
    }
}
