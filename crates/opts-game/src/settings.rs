//! Persistent user configuration: file I/O for [`GameSettings`].
//!
//! The settings type, its defaults, the session overlay and the baseline live in the
//! `opts-game-state` crate and are re-exported here. This module keeps the loading of
//! `settings.toml` and `settings.db`, the saves and the background writer. They are free
//! functions, because an inherent `impl` block cannot be written in another crate.
//!
//! # Storage Architecture
//! - **`settings.toml`**: Immutable baseline reference. Read on startup, never overwritten by application logic.
//! - **`settings.db`**: Authoritative SQLite database storing the active user preferences.

pub use opts_game_state::settings::{Baseline, GameSettings};

mod load;

mod persistence;

pub use load::*;
pub use persistence::*;

#[cfg(test)]
mod tests;
