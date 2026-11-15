//! Date, time and weather controls shared by the wizard's map step, its *When* step and the pause
//! menu's *Environment* page: the stepper arithmetic, fixed-size `[-] value [+]` steppers and the
//! frame of a choice chip.
//!
//! The arithmetic works on [`CalendarDate`] and an hour/minute pair, so the wizard
//! (`NewGameSetup`) and the running session (`SessionEnvironment`) step the same way. Labels use
//! the wizard's message keys (`wiz-when-*`, `wiz-month-*`).

mod arith;
mod choice;
mod field;
mod stepper;

pub use arith::{clamp_date, date_value, month_key, step_date, step_time, time_value};
pub use choice::{ChoiceMark, show_chosen, spawn_choice_frame};
pub use field::{Field, YEARS};
pub use stepper::{spawn_stepper, spawn_stepper_row};

#[cfg(test)]
mod tests;
