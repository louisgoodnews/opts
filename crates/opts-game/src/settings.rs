//! Persistent user configuration.
//!
//! The settings type, its defaults, the session overlay and the baseline live in the
//! `opts-game-state` crate and are re-exported here. The file I/O (`settings.toml`,
//! `settings.db`, the saves and the background writer) is the crate `opts-settings-io`
//! (ISS-068 remainder), re-exported here at the old paths.

pub use opts_game_state::settings::{Baseline, GameSettings};

pub use opts_settings_io::*;

#[cfg(test)]
mod tests;
