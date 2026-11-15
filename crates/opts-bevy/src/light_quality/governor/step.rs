//! The judgement of one frame: the level step over the sliding window.

use super::*;

/// Feeds one frame (`frame_ms`, taking `dt` seconds of wall time) to the governor, judged
/// against the deadline `target_ms`.
///
/// * The first `warmup_s` seconds after the gate opened are ignored.
/// * A frame longer than `hitch_ms` is a hitch (a tile streamed in): it is neither counted in the
///   window nor in the timers, unless `hitch_run_limit` such frames follow each other.
/// * A frame is **late** above `late_factor * target_ms`. The judgement needs a full window.
///   At least `down_late_share` late frames for `down_after_s` move one level down (steps are at
///   least `min_step_interval_s` apart).
/// * A level up is a probation: after `up_wait_s` seconds with no late frame in a full window
///   the governor tries one level up. If the level goes down again within `probation_s` the wait
///   doubles (`up_after_s`, 2x, 4x ... up to `up_after_max_s`); after `backoff_reset_s` without a
///   level down it is `up_after_s` again. Nothing is inferred from short frames, so this works
///   under vsync where a frame can never be shorter than the refresh interval.
/// * The level stays in `0..=5`; after a step the window starts again.
pub fn step(
    state: &mut GovernorState,
    cfg: &GovernorSettings,
    target_ms: f32,
    frame_ms: f32,
    dt: f32,
) -> Option<Change> {
    state.target_ms = target_ms;
    state.since_step_s = (state.since_step_s + dt).min(1.0e9);
    state.since_up_s = (state.since_up_s + dt).min(1.0e9);
    state.since_down_s = (state.since_down_s + dt).min(1.0e9);
    if state.since_down_s >= cfg.backoff_reset_s {
        state.up_wait_s = cfg.up_after_s;
    }
    if state.warmup_s < cfg.warmup_s {
        state.warmup_s += dt;
        return None;
    }
    if frame_ms > cfg.hitch_ms {
        state.hitch_run += 1;
        if state.hitch_run <= cfg.hitch_run_limit {
            return None;
        }
    } else {
        state.hitch_run = 0;
    }
    state.window.push_back(frame_ms);
    while state.window.len() > cfg.window_frames {
        state.window.pop_front();
    }
    if state.window.len() < cfg.window_frames {
        return None;
    }
    let late_above = cfg.late_factor * target_ms;
    let late = state.window.iter().filter(|ms| **ms > late_above).count();
    let share = late as f32 / state.window.len() as f32;
    state.late_share = share;
    state.over_s = if share >= cfg.down_late_share {
        state.over_s + dt
    } else {
        0.0
    };
    state.clean_s = if late == 0 { state.clean_s + dt } else { 0.0 };
    let ready = state.since_step_s >= cfg.min_step_interval_s;
    let from = state.level;
    let to = if ready && state.over_s >= cfg.down_after_s && from < MAX_LEVEL {
        if state.since_up_s < cfg.probation_s {
            state.up_wait_s = (state.up_wait_s * 2.0).min(cfg.up_after_max_s);
        }
        state.since_down_s = 0.0;
        from + 1
    } else if ready && state.clean_s >= state.up_wait_s && from > 0 {
        state.since_up_s = 0.0;
        from - 1
    } else {
        return None;
    };
    state.level = to;
    state.restart_window();
    Some(Change {
        from,
        to,
        target_ms,
        late_share: share,
    })
}
