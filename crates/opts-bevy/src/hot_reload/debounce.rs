//! Trailing debounce of file events: a path is released once it has been quiet for the window, or
//! after the cap if it never gets quiet.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// Longest a path is held back however busy it is.
pub const MAX_HOLD: Duration = Duration::from_secs(2);

/// Collects file events and releases them in bursts.
#[derive(Debug)]
pub struct Debouncer {
    window: Duration,
    pending: HashMap<PathBuf, (Instant, Instant)>,
}

impl Debouncer {
    /// A debouncer with the given quiet period.
    #[must_use]
    pub fn new(window: Duration) -> Self {
        Self {
            window,
            pending: HashMap::new(),
        }
    }

    /// Changes the quiet period.
    pub fn set_window(&mut self, window: Duration) {
        self.window = window;
    }

    /// Records an event for `path` at `now`.
    pub fn push(&mut self, path: PathBuf, now: Instant) {
        self.pending
            .entry(path)
            .and_modify(|(_, last)| *last = now)
            .or_insert((now, now));
    }

    /// Paths that are ready at `now`, removed from the debouncer (sorted for determinism).
    pub fn drain_ready(&mut self, now: Instant) -> Vec<PathBuf> {
        let window = self.window;
        let mut ready: Vec<PathBuf> = self
            .pending
            .iter()
            .filter(|(_, (first, last))| {
                now.duration_since(*last) >= window || now.duration_since(*first) >= MAX_HOLD
            })
            .map(|(p, _)| p.clone())
            .collect();
        ready.sort();
        for p in &ready {
            self.pending.remove(p);
        }
        ready
    }

    /// Whether anything is waiting.
    #[must_use]
    pub fn is_idle(&self) -> bool {
        self.pending.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(t0: Instant, n: u64) -> Instant {
        t0 + Duration::from_millis(n)
    }

    #[test]
    fn bursts_collapse_into_one_release() {
        let t0 = Instant::now();
        let mut d = Debouncer::new(Duration::from_millis(250));
        for i in 0..5 {
            d.push(PathBuf::from("a.sco"), ms(t0, i * 50));
        }
        assert!(
            d.drain_ready(ms(t0, 300)).is_empty(),
            "still within the window of the last event"
        );
        assert_eq!(d.drain_ready(ms(t0, 460)), vec![PathBuf::from("a.sco")]);
        assert!(d.is_idle());
    }

    #[test]
    fn paths_are_independent() {
        let t0 = Instant::now();
        let mut d = Debouncer::new(Duration::from_millis(100));
        d.push(PathBuf::from("a"), t0);
        d.push(PathBuf::from("b"), ms(t0, 90));
        assert_eq!(d.drain_ready(ms(t0, 120)), vec![PathBuf::from("a")]);
        assert_eq!(d.drain_ready(ms(t0, 200)), vec![PathBuf::from("b")]);
    }

    #[test]
    fn a_never_quiet_file_is_released_at_the_cap() {
        let t0 = Instant::now();
        let mut d = Debouncer::new(Duration::from_millis(250));
        for i in 0..30 {
            d.push(PathBuf::from("log.cfg"), ms(t0, i * 100));
        }
        assert_eq!(d.drain_ready(ms(t0, 2100)), vec![PathBuf::from("log.cfg")]);
    }
}
