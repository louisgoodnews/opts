//! Settings file I/O: the immutable reference `settings.toml`, the SQLite database `settings.db`
//! and the background writer (split out of `opts-game`, ISS-068 remainder).
//!
//! The settings type, its defaults, the session overlay and the baseline live in the
//! `opts-game-state` crate. This crate holds the file and SQLite I/O only, as free functions,
//! because an inherent `impl` block cannot be written in another crate.
//!
//! # Storage Architecture
//! - **`settings.toml`**: Immutable baseline reference. Read on startup, never overwritten by application logic.
//! - **`settings.db`**: Authoritative SQLite database storing the active user preferences.

mod load;
mod persistence;

pub use load::*;
pub use persistence::*;
