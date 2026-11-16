//! Unit tests for `super` (`jobs.rs`).

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
