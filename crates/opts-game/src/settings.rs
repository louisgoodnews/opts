//! Persistent user configuration and settings for OPTS.
//!
//! # Storage Architecture
//! - **`settings.toml`**: Immutable baseline reference. Read on startup, never overwritten by application logic.
//! - **`settings.db`**: Authoritative SQLite database storing active user preferences, per-widget colorizations, and custom themes.

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
    /// Whether dark mode is enabled (defaults to false for M3 baseline light).
    #[serde(default)]
    pub is_dark_theme: bool,
    /// Name of the active theme preset (e.g. "Default", "Amber", "Violet", "Teal").
    #[serde(default = "default_active_theme")]
    pub active_theme: String,
    /// Vehicle catalogue discovery interval in days (default: 1 = daily).
    #[serde(default = "default_discovery_interval_days")]
    pub discovery_interval_days: u32,
    /// Per-widget color customizations: maps `widget_type.role` -> hex color string.
    #[serde(default)]
    pub widget_colors: BTreeMap<String, String>,
    /// Quality of texture filtering (Off/4x/8x/16x); default 16x. Absent in older databases.
    #[serde(default)]
    pub texture_filtering: opts_bevy::TextureFiltering,
    /// The colour look (Vivid/Natural/Bevy default); default Vivid. Absent in older databases.
    #[serde(default)]
    pub look: crate::look::Look,
    /// The values as they were on disk when this process loaded them. Saving writes only what
    /// differs, so an instance that merely exits (or loaded stale values while another instance
    /// changed the file) cannot undo another instance's choices, e.g. the theme.
    #[serde(skip)]
    baseline: Baseline,
}

/// Snapshot of the persisted key/value pairs (the baseline of `GameSettings`, used to save only changed keys).
#[derive(Debug, Default)]
pub struct Baseline(std::sync::Mutex<Option<BTreeMap<String, String>>>);

impl Clone for Baseline {
    fn clone(&self) -> Self {
        Self(std::sync::Mutex::new(
            self.0.lock().ok().and_then(|g| g.clone()),
        ))
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

fn default_discovery_interval_days() -> u32 {
    1
}

impl Default for GameSettings {
    fn default() -> Self {
        Self {
            recent_map_path: None,
            omsi_path: None,
            is_dark_theme: false,
            active_theme: "Default".to_string(),
            discovery_interval_days: 1,
            widget_colors: BTreeMap::new(),
            texture_filtering: opts_bevy::TextureFiltering::default(),
            look: crate::look::Look::default(),
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
                Ok(settings) => {
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
    pub fn init_database(conn: &Connection) -> Result<(), rusqlite::Error> {
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS app_settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS widget_color_overrides (
                widget_key TEXT PRIMARY KEY,
                hex_color TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS custom_themes (
                name TEXT PRIMARY KEY,
                theme_json TEXT NOT NULL
            );
            "#,
        )?;
        Ok(())
    }

    /// Applies configuration overrides stored in an SQLite connection onto `self`.
    pub fn apply_from_connection(&mut self, conn: &Connection) -> Result<(), rusqlite::Error> {
        Self::init_database(conn)?;

        // 1. Read app_settings key-values
        let mut stmt = conn.prepare("SELECT key, value FROM app_settings")?;
        let rows = stmt.query_map([], |row| {
            let key: String = row.get(0)?;
            let value: String = row.get(1)?;
            Ok((key, value))
        })?;

        for row in rows {
            let (key, value) = row?;
            match key.as_str() {
                "recent_map_path" => {
                    if value.trim().is_empty() {
                        self.recent_map_path = None;
                    } else {
                        self.recent_map_path = Some(PathBuf::from(value));
                    }
                }
                "omsi_path" => {
                    self.omsi_path = (!value.trim().is_empty()).then(|| PathBuf::from(value));
                }
                "is_dark_theme" => {
                    self.is_dark_theme = value.parse::<bool>().unwrap_or(false);
                }
                "active_theme" => {
                    self.active_theme = value;
                }
                "discovery_interval_days" => {
                    self.discovery_interval_days = value.parse::<u32>().unwrap_or(1);
                }
                "texture_filtering" => {
                    self.texture_filtering =
                        opts_bevy::TextureFiltering::parse(&value).unwrap_or_default();
                }
                "look" => {
                    self.look = crate::look::Look::parse(&value).unwrap_or_default();
                }
                _ => {}
            }
        }

        // 2. Read widget_color_overrides
        let mut widget_stmt =
            conn.prepare("SELECT widget_key, hex_color FROM widget_color_overrides")?;
        let widget_rows = widget_stmt.query_map([], |row| {
            let k: String = row.get(0)?;
            let v: String = row.get(1)?;
            Ok((k, v))
        })?;

        for row in widget_rows {
            let (k, v) = row?;
            self.widget_colors.insert(k, v);
        }

        Ok(())
    }

    /// Saves configuration state to an SQLite connection.
    pub fn save_to_connection(&self, conn: &mut Connection) -> Result<(), rusqlite::Error> {
        Self::init_database(conn)?;

        let tx = conn.transaction()?;

        // Write app_settings
        let map_str = self
            .recent_map_path
            .as_ref()
            .and_then(|p| p.to_str())
            .unwrap_or("");

        tx.execute(
            "INSERT INTO app_settings (key, value) VALUES ('recent_map_path', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![map_str],
        )?;

        tx.execute(
            "INSERT INTO app_settings (key, value) VALUES ('omsi_path', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![self
                .omsi_path
                .as_ref()
                .and_then(|p| p.to_str())
                .unwrap_or("")],
        )?;

        tx.execute(
            "INSERT INTO app_settings (key, value) VALUES ('is_dark_theme', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![self.is_dark_theme.to_string()],
        )?;

        tx.execute(
            "INSERT INTO app_settings (key, value) VALUES ('active_theme', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![&self.active_theme],
        )?;

        tx.execute(
            "INSERT INTO app_settings (key, value) VALUES ('discovery_interval_days', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![self.discovery_interval_days.to_string()],
        )?;

        for (k, v) in [
            ("texture_filtering", self.texture_filtering.key()),
            ("look", self.look.key()),
        ] {
            tx.execute(
                "INSERT INTO app_settings (key, value) VALUES (?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![k, v],
            )?;
        }

        // Write widget color overrides
        for (k, v) in &self.widget_colors {
            tx.execute(
                "INSERT INTO widget_color_overrides (widget_key, hex_color) VALUES (?1, ?2)
                 ON CONFLICT(widget_key) DO UPDATE SET hex_color = excluded.hex_color",
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

    /// The persisted values as key/value pairs (`widget:<key>` for colour overrides).
    fn entries(&self) -> BTreeMap<String, String> {
        let mut m = BTreeMap::new();
        m.insert(
            "recent_map_path".into(),
            self.recent_map_path
                .as_ref()
                .and_then(|p| p.to_str())
                .unwrap_or("")
                .to_string(),
        );
        m.insert(
            "omsi_path".into(),
            self.omsi_path
                .as_ref()
                .and_then(|p| p.to_str())
                .unwrap_or("")
                .to_string(),
        );
        m.insert("is_dark_theme".into(), self.is_dark_theme.to_string());
        m.insert("active_theme".into(), self.active_theme.clone());
        m.insert(
            "discovery_interval_days".into(),
            self.discovery_interval_days.to_string(),
        );
        m.insert(
            "texture_filtering".into(),
            self.texture_filtering.key().to_string(),
        );
        m.insert("look".into(), self.look.key().to_string());
        for (k, v) in &self.widget_colors {
            m.insert(format!("widget:{k}"), v.clone());
        }
        m
    }

    fn remember_baseline(&self) {
        if let Ok(mut b) = self.baseline.0.lock() {
            *b = Some(self.entries());
        }
    }

    /// Persists settings to the SQLite database without mutating the immutable `settings.toml`.
    ///
    /// Only the values that differ from what this process loaded (or saved last) are written:
    /// several instances share the file, and an instance must not overwrite a choice it did not
    /// make.
    pub fn save(&self) -> Result<(), rusqlite::Error> {
        let Some(path) = Self::database_path() else {
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
                        if let Some(widget) = k.strip_prefix("widget:") {
                            tx.execute(
                                "INSERT INTO widget_color_overrides (widget_key, hex_color) VALUES (?1, ?2)
                                 ON CONFLICT(widget_key) DO UPDATE SET hex_color = excluded.hex_color",
                                params![widget, v],
                            )?;
                        } else {
                            tx.execute(
                                "INSERT INTO app_settings (key, value) VALUES (?1, ?2)
                                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                                params![k, v],
                            )?;
                        }
                    }
                    tx.commit()?;
                }
                tracing::info!(changed = changed.len(), path = ?path, "Persisted changed user settings to SQLite database");
            }
        }
        self.remember_baseline();
        Ok(())
    }

    /// Helper to set or update a specific per-widget color override.
    pub fn set_widget_color(&mut self, widget_name: &str, role: &str, hex: &str) {
        let key = format!("{}.{}", widget_name, role);
        self.widget_colors.insert(key, hex.to_string());
    }

    /// Helper to save a single custom theme JSON into SQLite.
    pub fn save_custom_theme(&self, name: &str, theme_json: &str) -> Result<(), rusqlite::Error> {
        let Some(path) = Self::database_path() else {
            return Ok(());
        };
        let conn = Connection::open(&path)?;
        Self::init_database(&conn)?;
        conn.execute(
            "INSERT INTO custom_themes (name, theme_json) VALUES (?1, ?2)
             ON CONFLICT(name) DO UPDATE SET theme_json = excluded.theme_json",
            params![name, theme_json],
        )?;
        Ok(())
    }

    /// Helper to load a custom theme JSON from SQLite.
    pub fn load_custom_theme(name: &str) -> Option<String> {
        let path = Self::database_path()?;
        let conn = Connection::open(&path).ok()?;
        Self::init_database(&conn).ok()?;
        let mut stmt = conn
            .prepare("SELECT theme_json FROM custom_themes WHERE name = ?1")
            .ok()?;
        stmt.query_row(params![name], |row| row.get(0)).ok()
    }

    /// Synchronizes a `MaterialTheme` with active settings: mode (dark/light), preset, and widget overrides.
    pub fn apply_to_theme(&self, theme: &mut opts_gui::theme::theme::MaterialTheme) {
        let mut scheme = if self.is_dark_theme {
            opts_gui::theme::color::ColorScheme::dark()
        } else {
            opts_gui::theme::color::ColorScheme::light()
        };

        match self.active_theme.as_str() {
            "Amber" => {
                scheme.set(
                    opts_gui::theme::color::ColorRole::Primary,
                    Color::srgb(0.96, 0.62, 0.04),
                );
                scheme.set(
                    opts_gui::theme::color::ColorRole::Secondary,
                    Color::srgb(0.85, 0.45, 0.10),
                );
            }
            "Violet" => {
                scheme.set(
                    opts_gui::theme::color::ColorRole::Primary,
                    Color::srgb(0.55, 0.36, 0.96),
                );
                scheme.set(
                    opts_gui::theme::color::ColorRole::Secondary,
                    Color::srgb(0.40, 0.20, 0.80),
                );
            }
            "Emerald" => {
                scheme.set(
                    opts_gui::theme::color::ColorRole::Primary,
                    Color::srgb(0.06, 0.72, 0.51),
                );
                scheme.set(
                    opts_gui::theme::color::ColorRole::Secondary,
                    Color::srgb(0.04, 0.55, 0.38),
                );
            }
            "Teal" => {
                scheme.set(
                    opts_gui::theme::color::ColorRole::Primary,
                    Color::srgb(0.08, 0.65, 0.75),
                );
                scheme.set(
                    opts_gui::theme::color::ColorRole::Secondary,
                    Color::srgb(0.05, 0.50, 0.60),
                );
            }
            _ => {}
        }

        for (key, hex) in &self.widget_colors {
            if let Some(color) = Self::hex_to_color(hex) {
                if key.ends_with(".Primary") {
                    scheme.set(opts_gui::theme::color::ColorRole::Primary, color);
                } else if key.ends_with(".SecondaryContainer") {
                    scheme.set(opts_gui::theme::color::ColorRole::SecondaryContainer, color);
                }
            }
        }

        theme.scheme = scheme;
    }

    fn hex_to_color(hex: &str) -> Option<Color> {
        let hex = hex.strip_prefix('#').unwrap_or(hex);
        if hex.len() == 6 {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()? as f32 / 255.0;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()? as f32 / 255.0;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()? as f32 / 255.0;
            Some(Color::srgb(r, g, b))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn baseline_tracks_what_changed_since_the_load() {
        let mut s = GameSettings::default();
        s.remember_baseline();
        let before = s.baseline.0.lock().unwrap().clone().unwrap();
        s.is_dark_theme = true;
        let now = s.entries();
        let changed: Vec<_> = now
            .iter()
            .filter(|(k, v)| before.get(*k) != Some(*v))
            .collect();
        assert_eq!(changed.len(), 1);
        assert_eq!(changed[0].0, "is_dark_theme");
    }

    #[test]
    fn test_sqlite_settings_roundtrip() {
        let mut conn = Connection::open_in_memory().expect("In-memory SQLite connection");
        let mut settings = GameSettings {
            recent_map_path: Some(PathBuf::from("/maps/Berlin")),
            omsi_path: Some(PathBuf::from("/games/OMSI 2")),
            is_dark_theme: true,
            active_theme: "Amber".to_string(),
            discovery_interval_days: 1,
            widget_colors: BTreeMap::new(),
            texture_filtering: opts_bevy::TextureFiltering::X4,
            look: crate::look::Look::Natural,
            baseline: Baseline::default(),
        };
        settings.set_widget_color("Button", "Primary", "#FFB300");

        settings
            .save_to_connection(&mut conn)
            .expect("Saving to SQLite must succeed");

        let mut loaded = GameSettings::default();
        loaded
            .apply_from_connection(&conn)
            .expect("Loading from SQLite must succeed");

        assert_eq!(loaded.recent_map_path, Some(PathBuf::from("/maps/Berlin")));
        assert_eq!(loaded.omsi_path, Some(PathBuf::from("/games/OMSI 2")));
        assert!(loaded.is_dark_theme);
        assert_eq!(loaded.active_theme, "Amber");
        assert_eq!(loaded.texture_filtering, opts_bevy::TextureFiltering::X4);
        assert_eq!(loaded.look, crate::look::Look::Natural);
        assert_eq!(
            loaded.widget_colors.get("Button.Primary"),
            Some(&"#FFB300".to_string())
        );
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
