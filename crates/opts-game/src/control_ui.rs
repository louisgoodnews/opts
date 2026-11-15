//! Game-side UI introspection of the control protocol: `ui.screen` and `ui.wait_for`.
//!
//! `ui.screen` answers "where am I and what can I do now" in one call (app state, shell screen,
//! open dialogs with their buttons, toasts, focus, scrollable views); `ui.wait_for` blocks a
//! script until a condition on the UI, the app or the drive holds, so no script has to sleep.
mod eval;
mod screen;
mod wait;

use opts_bevy::control::state::ControlRegistry;
pub use screen::screen_json;

/// Registers the methods.
pub fn register(r: &mut ControlRegistry) {
    r.register("ui.screen", screen::screen);
    r.register("ui.wait_for", wait::wait_for);
}

#[cfg(test)]
mod tests;
