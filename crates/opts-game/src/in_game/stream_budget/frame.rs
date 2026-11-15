//! Wall-clock budget of one frame of streaming work.

use std::time::{Duration, Instant};

/// Wall-clock budget of one frame for streaming work.
#[derive(Debug, Clone, Copy)]
pub struct FrameBudget {
    limit: Duration,
    start: Instant,
    started: u32,
}

impl FrameBudget {
    /// A budget of `limit` starting now.
    #[must_use]
    pub fn new(limit: Duration) -> Self {
        Self {
            limit,
            start: Instant::now(),
            started: 0,
        }
    }

    /// Whether another work item may start: always the first one of a frame (progress guarantee),
    /// afterwards while the elapsed time is below the limit. Call once per item you start.
    pub fn allow(&mut self) -> bool {
        let ok = self.started == 0 || self.start.elapsed() < self.limit;
        if ok {
            self.started += 1;
        }
        ok
    }

    /// Items allowed so far.
    #[must_use]
    pub fn started(&self) -> u32 {
        self.started
    }

    /// Time used so far.
    #[must_use]
    pub fn used(&self) -> Duration {
        self.start.elapsed()
    }
}
