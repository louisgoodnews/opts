//! Unit tests of `light_quality`.

use bevy::prelude::*;
use bevy::time::Real;
use opts_light_quality::*;

mod ladder;

/// 60 Hz deadline.
const T60: f32 = 1000.0 / 60.0;

/// The default thresholds without the start-up warm-up (tested on its own).
fn cfg() -> GovernorSettings {
    GovernorSettings {
        warmup_s: 0.0,
        ..GovernorSettings::default()
    }
}

/// Feeds `seconds` of frames given by `ms(i)`; returns (time, change) pairs.
fn feed_with(
    state: &mut GovernorState,
    cfg: &GovernorSettings,
    target: f32,
    seconds: f32,
    ms: impl Fn(usize) -> f32,
) -> Vec<(f32, Change)> {
    let mut t = 0.0;
    let mut i = 0;
    let mut out = Vec::new();
    while t < seconds {
        let m = ms(i);
        let dt = m / 1000.0;
        t += dt;
        i += 1;
        out.extend(step(state, cfg, target, m, dt).map(|c| (t, c)));
    }
    out
}

/// Frames of `ms` each for `seconds`, judged against `target`.
fn feed_at(
    state: &mut GovernorState,
    cfg: &GovernorSettings,
    target: f32,
    ms: f32,
    seconds: f32,
) -> Vec<Change> {
    feed_with(state, cfg, target, seconds, |_| ms)
        .into_iter()
        .map(|(_, c)| c)
        .collect()
}

#[test]
fn vsync_at_sixty_hertz_never_steps_down() {
    let mut s = GovernorState::default();
    // Every frame 16.7 ms, a little jitter, 5 minutes: not a single late frame.
    let changes = feed_with(&mut s, &cfg(), T60, 300.0, |i| 16.7 + (i % 3) as f32 * 0.4);
    assert!(changes.is_empty(), "{changes:?}");
    assert_eq!(s.level, 0);
    assert_eq!(s.late_share, 0.0);
}

#[test]
fn a_quarter_of_the_frames_a_vblank_late_steps_down_once_per_five_seconds_to_five() {
    let mut s = GovernorState::default();
    let changes = feed_with(&mut s, &cfg(), T60, 60.0, |i| {
        if i % 4 == 3 {
            33.3
        } else {
            16.7
        }
    });
    assert_eq!(changes.len(), 5, "{changes:?}");
    assert!(changes.iter().all(|(_, c)| c.to == c.from + 1));
    for w in changes.windows(2) {
        assert!(w[1].0 - w[0].0 >= 5.0 - 0.05, "{changes:?}");
    }
    assert!((changes[0].1.late_share - 0.25).abs() < 0.02);
    assert!((changes[0].1.target_ms - T60).abs() < 1e-3);
    assert_eq!(s.level, 5);
    // And it stays there.
    let more = feed_with(&mut s, &cfg(), T60, 60.0, |i| {
        if i % 4 == 3 {
            33.3
        } else {
            16.7
        }
    });
    assert!(more.is_empty());
}

#[test]
fn nine_percent_late_is_below_the_threshold() {
    let mut s = GovernorState::default();
    // 11 of 120 frames late = 9.2 %.
    let changes = feed_with(&mut s, &cfg(), T60, 60.0, |i| {
        if i % 120 < 11 {
            33.3
        } else {
            16.7
        }
    });
    assert!(changes.is_empty(), "{changes:?}");
}

#[test]
fn it_needs_a_full_window_and_two_seconds() {
    let mut s = GovernorState::default();
    // All frames late (40 ms): the 120-frame window is full after 4.8 s, plus 2 s.
    let changes = feed_with(&mut s, &cfg(), T60, 10.0, |_| 40.0);
    assert_eq!(changes.len(), 1, "{changes:?}");
    assert!(changes[0].0 > 6.5 && changes[0].0 < 7.2, "{changes:?}");
}

#[test]
fn recovery_steps_up_after_twenty_clean_seconds_not_before() {
    let mut s = GovernorState::at(3);
    // The window fills first (2 s), then 20 s clean.
    assert!(
        feed_at(&mut s, &cfg(), T60, 16.7, 21.0).is_empty(),
        "not yet"
    );
    assert_eq!(s.level, 3);
    let c = feed_at(&mut s, &cfg(), T60, 16.7, 3.0);
    assert_eq!(c.len(), 1);
    assert_eq!((c[0].from, c[0].to), (3, 2));
    assert_eq!(c[0].late_share, 0.0);
}

#[test]
fn a_single_late_frame_restarts_the_clean_time() {
    let mut s = GovernorState::at(3);
    let c = feed_with(&mut s, &cfg(), T60, 40.0, |i| {
        if i % 1000 == 999 {
            33.3
        } else {
            16.7
        }
    });
    // About 1000 frames (16.7 s) per late frame: a clean 20 s never completes.
    assert!(c.is_empty(), "{c:?}");
}

mod backoff;
mod budget;
mod system;
