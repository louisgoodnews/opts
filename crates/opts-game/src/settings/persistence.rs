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

    /// The persisted values as key/value pairs.
    pub(super) fn entries(&self) -> BTreeMap<String, String> {
        let path = |p: &Option<PathBuf>| {
            p.as_ref()
                .and_then(|p| p.to_str())
                .unwrap_or("")
                .to_string()
        };
        let mut m = BTreeMap::new();
        m.insert("recent_map_path".into(), path(&self.recent_map_path));
        m.insert("omsi_path".into(), path(&self.omsi_path));
        m.insert(
            "discovery_interval_days".into(),
            self.discovery_interval_days.to_string(),
        );
        m.insert(
            "texture_filtering".into(),
            self.texture_filtering.key().to_string(),
        );
        m.insert("look".into(), self.look.key().to_string());
        m.extend(self.theme_entries());
        for (k, v) in &self.extra {
            m.entry(k.clone()).or_insert_with(|| v.clone());
        }
        // A profile's values are the session's, not the person's: the file keeps the global ones.
        for (k, global) in &self.session_overlay {
            match global {
                Some(v) => {
                    m.insert(k.clone(), v.clone());
                }
                None => {
                    m.remove(k);
                }
            }
        }
        m
    }

    pub(super) fn remember_baseline(&self) {
        if let Ok(mut b) = self.baseline.0.lock() {
            *b = Some(self.entries());
        }
    }

    /// Hands a copy of the settings to the single background writer: the caller never waits for
    /// SQLite (M-0088). Writes made in quick succession are merged: only the newest copy per
    /// settings file is written. [`GameSettings::flush_background_saves`] waits for the writer (shutdown).
    pub fn save_in_background(&self) {
        let saver = saver();
        let Ok(mut st) = saver.state.lock() else {
            return;
        };
        // Newer copies replace older ones for the same file only.
        match st
            .latest
            .iter_mut()
            .find(|s| s.db_path_override == self.db_path_override)
        {
            Some(slot) => *slot = self.clone(),
            None => st.latest.push(self.clone()),
        }
        if st.running {
            return;
        }
        st.running = true;
        drop(st);
        let worker = saver.clone();
        let spawned = std::thread::Builder::new()
            .name("settings-writer".into())
            .spawn(move || loop {
                let next = {
                    let Ok(mut st) = worker.state.lock() else {
                        return;
                    };
                    let next = std::mem::take(&mut st.latest);
                    if next.is_empty() {
                        st.running = false;
                        worker.idle.notify_all();
                    }
                    next
                };
                if next.is_empty() {
                    return;
                }
                for settings in next {
                    if let Err(err) = settings.save() {
                        tracing::warn!(err = ?err, "could not save settings");
                    }
                }
            });
        if let Err(err) = spawned {
            tracing::warn!(err = %err, "cannot start the settings writer; saving here");
            if let Ok(mut st) = saver.state.lock() {
                st.running = false;
                st.latest.clear();
            }
            let _ = self.save();
        }
    }

    /// Waits until the background writer has written everything handed to it.
    pub fn flush_background_saves() {
        let saver = saver();
        let Ok(mut st) = saver.state.lock() else {
            return;
        };
        while st.running {
            st = match saver.idle.wait(st) {
                Ok(g) => g,
                Err(_) => return,
            };
        }
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

/// The state shared between the callers of [`GameSettings::save_in_background`] and the writer.
struct Saver {
    state: std::sync::Mutex<SaverState>,
    idle: std::sync::Condvar,
}

#[derive(Default)]
struct SaverState {
    latest: Vec<GameSettings>,
    running: bool,
}

fn saver() -> std::sync::Arc<Saver> {
    static SAVER: std::sync::OnceLock<std::sync::Arc<Saver>> = std::sync::OnceLock::new();
    SAVER
        .get_or_init(|| {
            std::sync::Arc::new(Saver {
                state: std::sync::Mutex::new(SaverState::default()),
                idle: std::sync::Condvar::new(),
            })
        })
        .clone()
}
