//! Persistent user configuration and settings for OPTS.
//!
//! # Storage Architecture
//! - **`settings.toml`**: Immutable baseline reference. Read on startup, never overwritten by application logic.
//! - **`settings.db`**: Authoritative SQLite database storing the active user preferences.
//!
//! Theme and interface keys (M-0061): `theme_mode` (`dark`/`light`/`system`/`auto`, default dark),
//! `auto_light_from` / `auto_dark_from` (`HH:MM`, 07:00 / 19:00), `high_contrast`, `active_theme`
//! (the accent preset), `theme_file` (a RON theme file, see `theme_files`), `ui_scale` (75-200 percent), `density`, `reduced_motion`. A database
//! without `theme_mode` is migrated from the old `is_dark_theme` key (true: dark, false: light);
//! with neither key the theme is dark. The per-widget colour overrides and the `custom_themes`
//! table of earlier versions are no longer read (theme files in the config directory replace
//! them, see `theme_files`).

use bevy::prelude::*;
use directories::ProjectDirs;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

/// Application settings persisted to an SQLite database with an immutable TOML reference.
#[derive(Resource, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GameSettings {
    /// Path to the most recently loaded map directory containing `picture.jpg`.
    #[serde(default)]
    pub recent_map_path: Option<PathBuf>,
    /// OMSI 2 installation chosen by the user (or found at a known location) and remembered for
    /// later runs; `--omsi-path` and `$OMSI_PATH` override it.
    #[serde(default)]
    pub omsi_path: Option<PathBuf>,
    /// The legacy light/dark flag of versions before M-0061. Only read, to migrate a settings
    /// file without `theme_mode`; never written.
    #[serde(default, skip_serializing)]
    pub is_dark_theme: Option<bool>,
    /// How the theme is chosen: dark (default), light, system or auto by the clock.
    #[serde(default)]
    pub theme_mode: opts_gui::theme::ThemeMode,
    /// Local time (`HH:MM`) the light theme starts in the auto mode.
    #[serde(default = "default_light_from")]
    pub auto_light_from: String,
    /// Local time (`HH:MM`) the dark theme starts in the auto mode.
    #[serde(default = "default_dark_from")]
    pub auto_dark_from: String,
    /// Use the high-contrast theme (overrides `theme_mode`; never switched automatically).
    #[serde(default)]
    pub high_contrast: bool,
    /// Name of the accent preset (`Default` = Transit blue, `Amber`, `Violet`, `Emerald`, `Teal`).
    #[serde(default = "default_active_theme")]
    pub active_theme: String,
    /// Name (file stem) of the theme file in the `themes/` folder of the configuration directory
    /// to use, empty for none.
    #[serde(default)]
    pub theme_file: String,
    /// Interface scale in percent (75-200).
    #[serde(default = "default_ui_scale")]
    pub ui_scale: u32,
    /// Information density of the interface.
    #[serde(default)]
    pub density: opts_gui::theme::Density,
    /// Reduce motion (transitions become instant); the OS hint is added to this.
    #[serde(default)]
    pub reduced_motion: bool,
    /// Vehicle catalogue discovery interval in days (default: 1 = daily).
    #[serde(default = "default_discovery_interval_days")]
    pub discovery_interval_days: u32,
    /// Quality of texture filtering (Off/4x/8x/16x); default 16x. Absent in older databases.
    #[serde(default)]
    pub texture_filtering: opts_bevy::TextureFiltering,
    /// The colour look (Vivid/Natural/Bevy default); default Vivid. Absent in older databases.
    #[serde(default)]
    pub look: crate::look::Look,
    /// Settings registered through the settings registry that have no typed field: key to stored
    /// text (see `docs/src/architecture/settings-registry.md`). Unknown keys of a database are
    /// kept here too, so a newer version's keys survive.
    #[serde(default)]
    pub extra: BTreeMap<String, String>,
    /// Keys that were present in the database this process loaded (used by `migrate_from`).
    #[serde(skip)]
    pub loaded_keys: std::collections::BTreeSet<String>,
    /// Where `save` writes instead of the user's configuration directory (tests).
    #[serde(skip)]
    pub db_path_override: Option<PathBuf>,
    /// The values as they were on disk when this process loaded them. Saving writes only what
    /// differs, so an instance that merely exits (or loaded stale values while another instance
    /// changed the file) cannot undo another instance's choices, e.g. the theme.
    #[serde(skip)]
    baseline: Baseline,
}

/// Snapshot of the persisted key/value pairs (the baseline of `GameSettings`, used to save only changed keys).
#[derive(Debug, Default)]
pub struct Baseline(std::sync::Arc<std::sync::Mutex<Option<BTreeMap<String, String>>>>);

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

fn default_active_theme() -> String {
    "Default".to_string()
}

fn default_light_from() -> String {
    "07:00".to_string()
}

fn default_dark_from() -> String {
    "19:00".to_string()
}

fn default_ui_scale() -> u32 {
    100
}

fn default_discovery_interval_days() -> u32 {
    1
}

impl Default for GameSettings {
    fn default() -> Self {
        Self {
            recent_map_path: None,
            omsi_path: None,
            is_dark_theme: None,
            theme_mode: opts_gui::theme::ThemeMode::Dark,
            auto_light_from: default_light_from(),
            auto_dark_from: default_dark_from(),
            high_contrast: false,
            active_theme: "Default".to_string(),
            theme_file: String::new(),
            ui_scale: 100,
            density: opts_gui::theme::Density::Comfortable,
            reduced_motion: false,
            discovery_interval_days: 1,
            texture_filtering: opts_bevy::TextureFiltering::default(),
            look: crate::look::Look::default(),
            extra: BTreeMap::new(),
            loaded_keys: std::collections::BTreeSet::new(),
            db_path_override: None,
            baseline: Baseline::default(),
        }
    }
}

impl GameSettings {
    /// Resolves the immutable reference configuration file path (`settings.toml`).
    pub fn toml_reference_path() -> Option<PathBuf> {
        ProjectDirs::from("org", "opts", "opts").map(|dirs| dirs.config_dir().join("settings.toml"))
    }

    /// Resolves the authoritative SQLite database file path (`settings.db`).
    pub fn database_path() -> Option<PathBuf> {
        ProjectDirs::from("org", "opts", "opts").map(|dirs| dirs.config_dir().join("settings.db"))
    }

    /// Loads the immutable baseline configuration from `settings.toml` if present.
    pub fn load_immutable_toml_reference() -> Option<Self> {
        let path = Self::toml_reference_path()?;
        if !path.exists() {
            return None;
        }

        match fs::read_to_string(&path) {
            Ok(content) => match toml::from_str::<Self>(&content) {
                Ok(mut settings) => {
                    let has_mode = content
                        .parse::<toml::Table>()
                        .is_ok_and(|t| t.contains_key("theme_mode"));
                    settings.migrate_legacy_theme(has_mode);
                    tracing::info!(path = ?path, "Loaded immutable baseline settings from settings.toml");
                    Some(settings)
                }
                Err(err) => {
                    tracing::warn!(err = ?err, path = ?path, "Failed to parse immutable settings.toml reference");
                    None
                }
            },
            Err(err) => {
                tracing::warn!(err = ?err, path = ?path, "Failed to read settings.toml reference file");
                None
            }
        }
    }

    /// Initializes required database tables in SQLite.
    ///
    /// The tables `widget_color_overrides` and `custom_themes` of earlier versions are left alone
    /// if they exist (nothing reads them any more) and are not created.
    pub fn init_database(conn: &Connection) -> Result<(), rusqlite::Error> {
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS app_settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            "#,
        )?;
        Ok(())
    }

    /// Applies configuration overrides stored in an SQLite connection onto `self`.
    pub fn apply_from_connection(&mut self, conn: &Connection) -> Result<(), rusqlite::Error> {
        Self::init_database(conn)?;

        let mut stmt = conn.prepare("SELECT key, value FROM app_settings")?;
        let rows = stmt.query_map([], |row| {
            let key: String = row.get(0)?;
            let value: String = row.get(1)?;
            Ok((key, value))
        })?;

        let mut has_mode = false;
        for row in rows {
            let (key, value) = row?;
            has_mode |= key == "theme_mode";
            self.apply_key(&key, &value);
            self.loaded_keys.insert(key);
        }
        self.migrate_legacy_theme(has_mode);
        Ok(())
    }

    /// The keys that have a typed field of their own; all others live in [`Self::extra`].
    pub const FIELD_KEYS: [&'static str; 14] = [
        "recent_map_path",
        "omsi_path",
        "discovery_interval_days",
        "texture_filtering",
        "look",
        "theme_mode",
        "auto_light_from",
        "auto_dark_from",
        "high_contrast",
        "active_theme",
        "theme_file",
        "ui_scale",
        "density",
        "reduced_motion",
    ];

    /// Applies one stored key (the loader's parser): a typed field, or [`Self::extra`] for any
    /// other key. Unreadable values of a field leave its default.
    pub fn apply_key(&mut self, key: &str, value: &str) {
        match key {
            "recent_map_path" => {
                self.recent_map_path = (!value.trim().is_empty()).then(|| PathBuf::from(value));
            }
            "omsi_path" => {
                self.omsi_path = (!value.trim().is_empty()).then(|| PathBuf::from(value));
            }
            "discovery_interval_days" => {
                self.discovery_interval_days = value.parse::<u32>().unwrap_or(1);
            }
            "texture_filtering" => {
                self.texture_filtering =
                    opts_bevy::TextureFiltering::parse(value).unwrap_or_default();
            }
            "look" => {
                self.look = crate::look::Look::parse(value).unwrap_or_default();
            }
            other => {
                if !self.apply_theme_key(other, value) {
                    self.extra.insert(other.to_string(), value.to_string());
                }
            }
        }
    }

    /// The stored text of `key` (a field or an extra key), `None` when nothing is stored.
    #[must_use]
    pub fn raw(&self, key: &str) -> Option<String> {
        if Self::FIELD_KEYS.contains(&key) {
            self.entries().get(key).cloned()
        } else {
            self.extra.get(key).cloned()
        }
    }

    /// Stores `value` under `key` without validation (see the registry for validated writes).
    pub fn set_raw(&mut self, key: &str, value: &str) {
        self.apply_key(key, value);
    }

    /// Removes an extra key (a typed field is reset by the registry writing its default).
    pub fn remove_extra(&mut self, key: &str) {
        self.extra.remove(key);
    }

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
    fn entries(&self) -> BTreeMap<String, String> {
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
        m
    }

    fn remember_baseline(&self) {
        if let Ok(mut b) = self.baseline.0.lock() {
            *b = Some(self.entries());
        }
    }

    /// Hands a copy of the settings to the single background writer: the caller never waits for
    /// SQLite (M-0088). Writes made in quick succession are merged: only the newest copy is
    /// written. [`GameSettings::flush_background_saves`] waits for the writer (shutdown).
    pub fn save_in_background(&self) {
        let saver = saver();
        let Ok(mut st) = saver.state.lock() else {
            return;
        };
        st.latest = Some(self.clone());
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
                    let next = st.latest.take();
                    if next.is_none() {
                        st.running = false;
                        worker.idle.notify_all();
                    }
                    next
                };
                let Some(settings) = next else { return };
                if let Err(err) = settings.save() {
                    tracing::warn!(err = ?err, "could not save settings");
                }
            });
        if let Err(err) = spawned {
            tracing::warn!(err = %err, "cannot start the settings writer; saving here");
            if let Ok(mut st) = saver.state.lock() {
                st.running = false;
                st.latest = None;
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
    latest: Option<GameSettings>,
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

#[cfg(test)]
mod tests {
    use super::*;
    use opts_gui::theme::{Density, ThemeMode};

    #[test]
    fn baseline_tracks_what_changed_since_the_load() {
        let mut s = GameSettings::default();
        s.remember_baseline();
        let before = s.baseline.0.lock().unwrap().clone().unwrap();
        s.theme_mode = ThemeMode::Light;
        let now = s.entries();
        let changed: Vec<_> = now
            .iter()
            .filter(|(k, v)| before.get(*k) != Some(*v))
            .collect();
        assert_eq!(changed.len(), 1);
        assert_eq!(changed[0].0, "theme_mode");
    }

    #[test]
    fn test_sqlite_settings_roundtrip() {
        let mut conn = Connection::open_in_memory().expect("In-memory SQLite connection");
        let settings = GameSettings {
            recent_map_path: Some(PathBuf::from("/maps/Berlin")),
            omsi_path: Some(PathBuf::from("/games/OMSI 2")),
            theme_mode: ThemeMode::AutoClock,
            auto_light_from: "06:30".into(),
            auto_dark_from: "20:15".into(),
            high_contrast: true,
            active_theme: "Amber".to_string(),
            ui_scale: 150,
            density: Density::Compact,
            reduced_motion: true,
            discovery_interval_days: 1,
            texture_filtering: opts_bevy::TextureFiltering::X4,
            look: crate::look::Look::Natural,
            ..GameSettings::default()
        };

        settings
            .save_to_connection(&mut conn)
            .expect("Saving to SQLite must succeed");

        let mut loaded = GameSettings::default();
        loaded
            .apply_from_connection(&conn)
            .expect("Loading from SQLite must succeed");

        assert_eq!(loaded.recent_map_path, Some(PathBuf::from("/maps/Berlin")));
        assert_eq!(loaded.omsi_path, Some(PathBuf::from("/games/OMSI 2")));
        assert_eq!(loaded.theme_mode, ThemeMode::AutoClock);
        assert_eq!(loaded.auto_light_from, "06:30");
        assert_eq!(loaded.auto_dark_from, "20:15");
        assert!(loaded.high_contrast);
        assert_eq!(loaded.active_theme, "Amber");
        assert_eq!(loaded.ui_scale, 150);
        assert_eq!(loaded.density, Density::Compact);
        assert!(loaded.reduced_motion);
        assert_eq!(loaded.texture_filtering, opts_bevy::TextureFiltering::X4);
        assert_eq!(loaded.look, crate::look::Look::Natural);
    }

    #[test]
    fn new_keys_default_when_absent_from_an_old_database() {
        // A database written before the keys existed: only the old ones are present.
        let conn = Connection::open_in_memory().expect("In-memory SQLite connection");
        GameSettings::init_database(&conn).expect("schema");
        conn.execute(
            "INSERT INTO app_settings (key, value) VALUES ('active_theme', 'Amber')",
            [],
        )
        .expect("insert");
        let mut s = GameSettings::default();
        s.apply_from_connection(&conn).expect("load");
        assert_eq!(s.active_theme, "Amber");
        assert_eq!(s.texture_filtering, opts_bevy::TextureFiltering::X16);
        assert_eq!(s.look, crate::look::Look::Vivid);
        // Neither theme_mode nor is_dark_theme: the default theme, dark.
        assert_eq!(s.theme_mode, ThemeMode::Dark);
        assert_eq!(s.ui_scale, 100);
        assert_eq!(s.density, Density::Comfortable);
        assert_eq!(s.auto_light_from, "07:00");
        assert_eq!(s.auto_dark_from, "19:00");
        // Unreadable values fall back to the defaults too.
        conn.execute(
            "INSERT INTO app_settings (key, value) VALUES ('look', 'garbage'), ('texture_filtering', '3')",
            [],
        )
        .expect("insert");
        s.apply_from_connection(&conn).expect("load");
        assert_eq!(s.look, crate::look::Look::Vivid);
        assert_eq!(s.texture_filtering, opts_bevy::TextureFiltering::X16);
    }

    #[test]
    fn changing_the_new_settings_marks_only_them_as_changed() {
        let mut s = GameSettings::default();
        s.remember_baseline();
        s.look = crate::look::Look::BevyDefault;
        s.texture_filtering = opts_bevy::TextureFiltering::Off;
        let before = s.baseline.0.lock().unwrap().clone().unwrap();
        let changed: Vec<_> = s
            .entries()
            .into_iter()
            .filter(|(k, v)| before.get(k) != Some(v))
            .map(|(k, _)| k)
            .collect();
        assert_eq!(changed, ["look", "texture_filtering"]);
    }
}
