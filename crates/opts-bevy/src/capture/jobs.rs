//! Limits, rate limiting and the one-job-at-a-time rule shared by bursts and recordings.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use bevy::prelude::Resource;
use serde_json::Value;

use super::error::CaptureError;

/// Hard and default limits (design: `screen-capture.md`, A5 and A7). A resource so tests can
/// shorten the time constants.
#[derive(Resource, Debug, Clone)]
pub struct CaptureLimits {
    /// Recording duration when `duration_ms` is omitted.
    pub default_duration_ms: u64,
    /// Longest recording.
    pub max_duration_ms: u64,
    /// Size limit when `max_size_mb` is omitted.
    pub default_size_mb: u64,
    /// Largest size limit.
    pub max_size_mb: u64,
    /// A recording stops, and none starts, below this much free disk space.
    pub min_free_disk_mb: u64,
    /// Most frames requested from the renderer and not yet delivered.
    pub max_in_flight: usize,
    /// Depth of the queue between the main thread and the encoder.
    pub queue_depth: usize,
    /// Below this frame rate the capture rate is halved.
    pub degrade_below_fps: f32,
    /// Below this frame rate for [`Self::stop_after`] the recording stops.
    pub stop_below_fps: f32,
    /// See [`Self::stop_below_fps`].
    pub stop_after: Duration,
    /// A full encoder queue for this long stops the recording.
    pub queue_full_after: Duration,
    /// In-flight frames with no delivery for this long stop the recording (window occluded).
    pub no_delivery_after: Duration,
    /// Job starts allowed per rolling minute.
    pub starts_per_minute: usize,
    /// Smallest gap between screenshots while a recording runs.
    pub screenshot_gap: Duration,
    /// How long `capture.record.stop` waits for the encoder to finish.
    pub finish_wait: Duration,
}

impl Default for CaptureLimits {
    fn default() -> Self {
        Self {
            default_duration_ms: 60_000,
            max_duration_ms: 30 * 60 * 1000,
            default_size_mb: 512,
            max_size_mb: 2048,
            min_free_disk_mb: 1024,
            max_in_flight: 3,
            queue_depth: 3,
            degrade_below_fps: 45.0,
            stop_below_fps: 30.0,
            stop_after: Duration::from_secs(5),
            queue_full_after: Duration::from_secs(3),
            no_delivery_after: Duration::from_secs(5),
            starts_per_minute: 10,
            screenshot_gap: Duration::from_millis(100),
            finish_wait: Duration::from_secs(10),
        }
    }
}

/// The result of a finished recording, kept for `status` and `stop`.
#[derive(Debug, Clone)]
pub struct Finished {
    /// Recording id.
    pub id: String,
    /// The `status`/`stop` payload.
    pub result: Value,
}

/// Capture job bookkeeping: the active job, recent results and the start rate.
#[derive(Resource, Default)]
pub struct CaptureJobs {
    busy: Arc<Mutex<Option<String>>>,
    starts: VecDeque<Instant>,
    /// Results of the last recordings, newest last (at most 8).
    pub finished: VecDeque<Finished>,
    /// When the last screenshot was taken.
    pub last_screenshot: Option<Instant>,
    counter: u32,
}

/// Held by a running job; dropping it frees the job slot, whatever way the job ends.
pub struct JobGuard(Arc<Mutex<Option<String>>>);

impl Drop for JobGuard {
    fn drop(&mut self) {
        if let Ok(mut g) = self.0.lock() {
            *g = None;
        }
    }
}

impl CaptureJobs {
    /// Takes the job slot for `kind` (`recording` or `burst`).
    ///
    /// # Errors
    /// `Busy` when a job is running or too many jobs started within the last minute.
    pub fn begin(
        &mut self,
        kind: &str,
        session: &str,
        limits: &CaptureLimits,
    ) -> Result<(String, JobGuard), CaptureError> {
        let mut slot = self.busy.lock().map_err(|_| poisoned())?;
        if let Some(active) = slot.as_ref() {
            return Err(CaptureError::Busy(format!(
                "another capture job is running ({active}); stop it or wait for it"
            )));
        }
        let now = Instant::now();
        while self
            .starts
            .front()
            .is_some_and(|t| now.duration_since(*t) > Duration::from_secs(60))
        {
            self.starts.pop_front();
        }
        if self.starts.len() >= limits.starts_per_minute {
            return Err(CaptureError::Busy(format!(
                "at most {} capture jobs may start per minute",
                limits.starts_per_minute
            )));
        }
        self.starts.push_back(now);
        self.counter += 1;
        let id = format!("{kind}-{session}-{}", self.counter);
        *slot = Some(id.clone());
        Ok((id, JobGuard(self.busy.clone())))
    }

    /// Whether a recording or burst is running.
    #[must_use]
    pub fn is_busy(&self) -> bool {
        self.busy.lock().map_or(true, |g| g.is_some())
    }

    /// Records a finished recording (keeps the last 8).
    pub fn finish(&mut self, f: Finished) {
        self.finished.retain(|x| x.id != f.id);
        self.finished.push_back(f);
        while self.finished.len() > 8 {
            self.finished.pop_front();
        }
    }

    /// The stored result of `id`.
    #[must_use]
    pub fn result(&self, id: &str) -> Option<&Value> {
        self.finished.iter().find(|f| f.id == id).map(|f| &f.result)
    }
}

fn poisoned() -> CaptureError {
    CaptureError::Failed("the capture job state is unavailable".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_job_at_a_time_and_the_slot_frees_on_drop() {
        let limits = CaptureLimits::default();
        let mut jobs = CaptureJobs::default();
        let (id, guard) = jobs.begin("recording", "abc123", &limits).expect("first");
        assert_eq!(id, "recording-abc123-1");
        assert!(matches!(
            jobs.begin("burst", "abc123", &limits),
            Err(CaptureError::Busy(_))
        ));
        drop(guard);
        assert!(jobs.begin("burst", "abc123", &limits).is_ok());
    }

    #[test]
    fn starts_are_rate_limited() {
        let limits = CaptureLimits {
            starts_per_minute: 2,
            ..CaptureLimits::default()
        };
        let mut jobs = CaptureJobs::default();
        for _ in 0..2 {
            let (_, g) = jobs.begin("burst", "s", &limits).expect("start");
            drop(g);
        }
        assert!(matches!(
            jobs.begin("burst", "s", &limits),
            Err(CaptureError::Busy(_))
        ));
    }
}
