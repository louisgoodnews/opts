//! Per-connection token-bucket rate limiter.

use std::time::Instant;

/// Requests per second and burst of the per-connection limiter.
const RATE_PER_SEC: f64 = 200.0;
const RATE_BURST: f64 = 400.0;

pub(super) struct Limiter {
    tokens: f64,
    last: Instant,
}

impl Limiter {
    pub(super) fn new() -> Self {
        Self {
            tokens: RATE_BURST,
            last: Instant::now(),
        }
    }

    pub(super) fn allow(&mut self) -> bool {
        let now = Instant::now();
        self.tokens = (self.tokens + now.duration_since(self.last).as_secs_f64() * RATE_PER_SEC)
            .min(RATE_BURST);
        self.last = now;
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}
