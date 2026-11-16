//! Background SQLite persistence and baseline diffing for [`GameSettings`].

use super::*;

/// Snapshot of the persisted key/value pairs (the baseline of `GameSettings`, used to save only changed keys).
#[derive(Debug, Default)]
pub struct Baseline(pub(super) std::sync::Arc<std::sync::Mutex<Option<BTreeMap<String, String>>>>);

/// Clones share the baseline: it describes the file, and a copy that is saved on a worker thread
/// ([`GameSettings::save_in_background`]) must update what the live settings compare against.
impl Clone for Baseline {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl PartialEq for Baseline {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl Eq for Baseline {}

mod background;
mod entries;

impl GameSettings {
    /// Saves configuration state to an SQLite connection (every key).
    pub fn save_to_connection(&self, conn: &mut Connection) -> Result<(), rusqlite::Error> {
        Self::init_database(conn)?;
        let tx = conn.transaction()?;
        for (k, v) in self.entries() {
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
    pub fn load_or_default() -> Self {
        let mut settings = Self::load_immutable_toml_reference().unwrap_or_default();

        if let Some(db_path) = Self::database_path() {
            if db_path.exists() {
                if let Ok(conn) = Connection::open(&db_path) {
                    if let Err(err) = settings.apply_from_connection(&conn) {
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
    pub fn save(&self) -> Result<(), rusqlite::Error> {
        let Some(path) = self.db_path_override.clone().or_else(Self::database_path) else {
            tracing::warn!("Unable to determine platform config directory for SQLite database");
            return Ok(());
        };

        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        let mut conn = Connection::open(&path)?;
        let now = self.entries();
        let before = self.baseline.0.lock().ok().and_then(|b| b.clone());
        match before {
            None => self.save_to_connection(&mut conn)?,
            Some(before) => {
                let changed: Vec<(&String, &String)> = now
                    .iter()
                    .filter(|(k, v)| before.get(*k) != Some(*v))
                    .collect();
                if !changed.is_empty() {
                    Self::init_database(&conn)?;
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
        self.remember_baseline();
        Ok(())
    }
}
