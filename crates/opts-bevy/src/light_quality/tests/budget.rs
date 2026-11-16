//! Unit tests of the frame budget: the target rates, vsync and the requested frame rate, and the level limits.

use super::*;

#[test]
fn vsync_off_at_144_hz_with_a_100_fps_target() {
    let target = target_ms(false, Some(144_000), 100);
    assert!((target - 10.0).abs() < 1e-4);
    // 7 ms frames (143 fps) are fine.
    let mut s = GovernorState::default();
    assert!(feed_at(&mut s, &cfg(), target, 7.0, 60.0).is_empty());
    // 14 ms frames (71 fps) are late (> 12.5 ms): steps down.
    let c = feed_at(&mut s, &cfg(), target, 14.0, 12.0);
    assert!(!c.is_empty());
    assert!((c[0].target_ms - 10.0).abs() < 1e-4);
    // 12 ms frames are not late.
    let mut s = GovernorState::default();
    assert!(feed_at(&mut s, &cfg(), target, 12.0, 60.0).is_empty());
}

#[test]
fn a_thirty_fps_target() {
    let target = target_ms(false, None, 30);
    assert!((target - 33.333).abs() < 0.01);
    let mut s = GovernorState::default();
    assert!(feed_at(&mut s, &cfg(), target, 33.3, 60.0).is_empty());
    assert!(!feed_at(&mut s, &cfg(), target, 45.0, 20.0).is_empty());
}

#[test]
fn the_budget_follows_vsync_the_monitor_and_the_requested_fps() {
    // vsync on: the monitor's interval; unknown refresh: 60 Hz.
    assert!((target_ms(true, Some(59_940), 60) - 16.683).abs() < 0.01);
    assert!((target_ms(true, None, 60) - 16.667).abs() < 0.01);
    // vsync on, 144 Hz, default target 60: the lower target wins.
    assert!((target_ms(true, Some(144_000), 60) - 16.667).abs() < 0.01);
    // vsync on, 30 Hz panel, target 60: the interval is longer than the wish.
    assert!((target_ms(true, Some(30_000), 60) - 33.333).abs() < 0.01);
    // vsync on, 144 Hz, target 240: not faster than the display.
    assert!((target_ms(true, Some(144_000), 240) - 6.944).abs() < 0.01);
    // vsync off ignores the monitor; the fps is clamped to 30..=240.
    assert!((target_ms(false, Some(144_000), 60) - 16.667).abs() < 0.01);
    assert!((target_ms(false, None, 5) - 33.333).abs() < 0.01);
    assert!((target_ms(false, None, 1000) - 4.1667).abs() < 0.01);
    assert!(is_vsync(bevy::window::PresentMode::Fifo));
    assert!(is_vsync(bevy::window::PresentMode::AutoVsync));
    assert!(!is_vsync(bevy::window::PresentMode::AutoNoVsync));
    assert!(!is_vsync(bevy::window::PresentMode::Immediate));
}

#[test]
fn level_zero_never_goes_up_and_five_never_down() {
    let mut s = GovernorState::default();
    assert!(feed_at(&mut s, &cfg(), T60, 5.0, 60.0).is_empty());
    assert_eq!(s.level, 0);
    let mut s = GovernorState::at(5);
    assert!(feed_at(&mut s, &cfg(), T60, 40.0, 60.0).is_empty());
    assert_eq!(s.level, 5);
}

#[test]
fn isolated_hitches_are_not_counted() {
    let cfg = cfg();
    let mut s = GovernorState::at(1);
    // Clean frames with a 120 ms hitch every second: the hitch is ignored, level 1 recovers.
    let mut changes = Vec::new();
    for _ in 0..40 {
        for i in 0..60 {
            let ms = if i == 30 { 120.0 } else { 16.7 };
            changes.extend(step(&mut s, &cfg, T60, ms, ms / 1000.0));
        }
    }
    assert_eq!(changes.len(), 1, "{changes:?}");
    assert_eq!(changes[0].to, 0);
    // A hitch frame never enters the window.
    let mut t = GovernorState::default();
    step(&mut t, &cfg, T60, 200.0, 0.2);
    assert_eq!(t.window_len(), 0);
}

#[test]
fn a_sustained_slowdown_is_not_mistaken_for_hitches() {
    let mut s = GovernorState::default();
    let changes = feed_at(&mut s, &cfg(), T60, 80.0, 30.0);
    assert!(
        !changes.is_empty(),
        "80 ms frames for 30 s must lower the level"
    );
}

#[test]
fn the_first_seconds_after_the_gate_opens_are_ignored() {
    let cfg = GovernorSettings::default();
    let mut s = GovernorState::default();
    // 4.9 s of terrible frames do not even enter the window.
    let c = feed_at(&mut s, &cfg, T60, 90.0, 4.9);
    assert!(c.is_empty());
    assert_eq!(s.window_len(), 0);
    // After a pause the warm-up applies again, the level stays.
    let mut s = GovernorState::at(2);
    feed_at(&mut s, &cfg, T60, 16.7, 6.0);
    assert!(s.window_len() > 0);
    s.pause();
    assert_eq!((s.level, s.window_len()), (2, 0));
    feed_at(&mut s, &cfg, T60, 16.7, 4.0);
    assert_eq!(s.window_len(), 0);
}

#[test]
fn a_step_restarts_the_window() {
    let cfg = cfg();
    let mut s = GovernorState::default();
    let c = feed_at(&mut s, &cfg, T60, 40.0, 7.5);
    assert_eq!(c.len(), 1);
    assert!(
        s.window_len() < cfg.window_frames,
        "the window starts again after a step"
    );
}
