//! Unit tests of the governor: the backoff of failed probations and the wait after a step.

use super::*;

/// Feeds frames of `ms` until the governor changes level (or `max_s` pass); returns the change.
fn until_change(state: &mut GovernorState, cfg: &GovernorSettings, ms: f32, max_s: f32) -> Change {
    let dt = ms / 1000.0;
    for _ in 0..(max_s / dt) as usize {
        if let Some(c) = step(state, cfg, T60, ms, dt) {
            return c;
        }
    }
    panic!(
        "no change within {max_s} s at {ms} ms (level {})",
        state.level
    );
}

#[test]
fn a_failed_probation_doubles_the_wait_up_to_160_seconds() {
    let cfg = cfg();
    let mut s = GovernorState::at(1);
    assert_eq!(s.up_wait_s, 20.0);
    let mut waits = Vec::new();
    for _ in 0..6 {
        // Clean until the governor tries a level up ...
        let up = until_change(&mut s, &cfg, 16.7, 400.0);
        assert_eq!((up.from, up.to), (1, 0));
        // ... and the lower level is too much: down again within the probation.
        let down = until_change(&mut s, &cfg, 40.0, 29.0);
        assert_eq!((down.from, down.to), (0, 1));
        waits.push(s.up_wait_s);
    }
    assert_eq!(waits, [40.0, 80.0, 160.0, 160.0, 160.0, 160.0]);
}

#[test]
fn the_wait_is_really_the_doubled_one() {
    let cfg = cfg();
    let mut s = GovernorState::at(1);
    until_change(&mut s, &cfg, 16.7, 400.0);
    until_change(&mut s, &cfg, 40.0, 29.0);
    assert_eq!(s.up_wait_s, 40.0);
    // Clean time needed: the 40 s wait, not 20 (the window refill is part of the clean time).
    assert!(feed_at(&mut s, &cfg, T60, 16.7, 38.0).is_empty());
    assert_eq!(feed_at(&mut s, &cfg, T60, 16.7, 4.0).len(), 1);
}

#[test]
fn the_backoff_resets_after_five_minutes_without_a_step_down() {
    let cfg = cfg();
    let mut s = GovernorState::at(1);
    until_change(&mut s, &cfg, 16.7, 400.0); // up
    until_change(&mut s, &cfg, 40.0, 29.0); // failed probation
    assert_eq!(s.up_wait_s, 40.0);
    // 5 minutes without a down-step (level 1 goes up again after 40 s meanwhile).
    feed_with(&mut s, &cfg, T60, 305.0, |_| 16.7);
    assert_eq!(s.up_wait_s, 20.0);
}

#[test]
fn a_down_step_long_after_an_up_step_is_no_failed_probation() {
    let cfg = cfg();
    let mut s = GovernorState::at(1);
    until_change(&mut s, &cfg, 16.7, 400.0); // up to 0
    feed_with(&mut s, &cfg, T60, 60.0, |_| 16.7);
    let down = until_change(&mut s, &cfg, 40.0, 29.0);
    assert_eq!(down.to, 1);
    assert_eq!(s.up_wait_s, 20.0);
}
