//! The notification service: the code is the crate `opts-notification` (ISS-068 remainder),
//! re-exported here at the old path `shell::notify`.
//!
//! The tests stay in this module (`notify/tests.rs`): they build the game's `ShellPlugin`, the
//! game's `AppState` and the test isolation, which `opts-notification` cannot depend on.

pub use opts_notification::*;

// The test module uses these names through `super::*`, as it did before the split.
#[cfg(test)]
use bevy::prelude::*;
#[cfg(test)]
use opts_gui::components::toast::ToastSeverity;
#[cfg(test)]
use opts_shell_nav::banner::BannerSpec;

#[cfg(test)]
mod tests;
