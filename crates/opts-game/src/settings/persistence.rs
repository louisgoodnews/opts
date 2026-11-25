//! Persistent user configuration: file I/O for [`GameSettings`].
//!
//! The settings type, its defaults, the session overlay and the baseline live in the
//! `opts-game-state` crate. This module keeps the loading of `settings.toml` and `settings.db`,
//! the saves and the background writer, as free functions: an inherent `impl` block cannot be
//! written in another crate.
//!
//! # Storage Architecture
//! - **`settings.toml`**: Immutable baseline reference. Read on startup, never overwritten by application logic.
//! - **`settings.db`**: Authoritative SQLite database storing the active user preferences.

use rusqlite::{params, Connection};

use opts_game_state::settings::GameSettings;

use super::load::{
    apply_from_connection, database_path, init_database, load_immutable_toml_reference,
};

mod background;

pub use background::{flush_background_saves, save_in_background};

/// Saves configuration state to an SQLite connection (every key).
pub fn save_to_connection(
    settings: &GameSettings,
    conn: &mut Connection,
) -> Result<(), rusqlite::Error> {
    init_database(conn)?;
    let tx = conn.transaction()?;
    for (k, v) in settings.entries() {
        tx.execute(
            "INSERT INTO app_settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![k, v],
        )?;
    }
    tx.commit()?;
    Ok(())
}

/// Loads settings from disk: reading `settings.toml` as immutable base, overlaid by `settings.db`.
pub fn load_or_default() -> GameSettings {
    let mut settings = load_immutable_toml_reference().unwrap_or_default();

    if let Some(db_path) = database_path() {
        if db_path.exists() {
            if let Ok(conn) = Connection::open(&db_path) {
                if let Err(err) = apply_from_connection(&mut settings, &conn) {
                    tracing::warn!(err = ?err, path = ?db_path, "Failed to query settings from SQLite");
                } else {
                    tracing::info!(path = ?db_path, "Applied user settings from SQLite database");
                }
            }
        }
    }

    settings.remember_baseline();
    settings
}

/// Persists settings to the SQLite database without mutating the immutable `settings.toml`.
///
/// Only the values that differ from what this process loaded (or saved last) are written:
/// several instances share the file, and an instance must not overwrite a choice it did not
/// make.
pub fn save(settings: &GameSettings) -> Result<(), rusqlite::Error> {
    let Some(path) = settings.db_path_override.clone().or_else(database_path) else {
        tracing::warn!("Unable to determine platform config directory for SQLite database");
        return Ok(());
    };

    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let mut conn = Connection::open(&path)?;
    let now = settings.entries();
    let before = settings.baseline.0.lock().ok().and_then(|b| b.clone());
    match before {
        None => save_to_connection(settings, &mut conn)?,
        Some(before) => {
            let changed: Vec<(&String, &String)> = now
                .iter()
                .filter(|(k, v)| before.get(*k) != Some(*v))
                .collect();
            if !changed.is_empty() {
                init_database(&conn)?;
                let tx = conn.transaction()?;
                for (k, v) in &changed {
                    tx.execute(
                        "INSERT INTO app_settings (key, value) VALUES (?1, ?2)
                         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                        params![k, v],
                    )?;
                }
                tx.commit()?;
            }
            tracing::info!(changed = changed.len(), path = ?path, "Persisted changed user settings to SQLite database");
        }
    }
    settings.remember_baseline();
    Ok(())
}
