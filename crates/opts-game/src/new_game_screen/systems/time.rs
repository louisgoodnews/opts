//! The time of the plan: the quick-time presets and the date and time steppers.

use crate::new_game_screen::planner::{quick_time_of, step_field, Field, QUICK_TIMES};

use super::Env;

/// The wall clock's hour and minute.
pub(super) fn wall_clock() -> (u32, u32) {
    use chrono::Timelike;
    let t = chrono::Local::now().time();
    (t.hour(), t.minute())
}

/// A quick time button: the clock of the preset (relative to the wall clock).
pub(super) fn quick_time(env: &mut Env, i: usize) {
    let (h, m) = quick_time_of(&QUICK_TIMES[i.min(4)], wall_clock());
    env.setup.hour = h;
    env.setup.minute = m;
    env.state.time_chosen = true;
    env.state.time_source = None;
}

/// A date or time stepper: changes the field and marks the date or the time as chosen.
pub(super) fn step_time(env: &mut Env, field: Field, delta: i32) {
    step_field(&mut env.setup, field, delta, false);
    if Field::DATE.contains(&field) {
        env.state.date_chosen = true;
    } else {
        env.state.time_chosen = true;
        env.state.time_source = None;
    }
}
