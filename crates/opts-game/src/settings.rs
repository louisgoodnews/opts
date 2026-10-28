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
    /// Keys a map profile overrides for this session, with the value they have globally
    /// (`None`: nothing is stored). Saving writes the global value of these keys, never the
    /// profile's, so a profile cannot reach the file ([`GameSettings::overlay_session`]).
    #[serde(skip)]
    session_overlay: BTreeMap<String, Option<String>>,
}

mod persistence;
pub use persistence::Baseline;

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
            session_overlay: BTreeMap::new(),
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

    /// Overrides keys for the running session (a map profile): the values are set in memory, the
    /// global values are remembered, and [`save`](Self::save) keeps writing the global ones.
    /// Applying again first restores what an earlier call overrode.
    pub fn overlay_session(&mut self, values: &BTreeMap<String, String>) {
        self.restore_session();
        for (key, value) in values {
            self.session_overlay.insert(key.clone(), self.raw(key));
            self.set_raw(key, value);
        }
    }

    /// Puts the global values back after [`overlay_session`](Self::overlay_session).
    pub fn restore_session(&mut self) {
        for (key, global) in std::mem::take(&mut self.session_overlay) {
            match global {
                Some(v) => self.set_raw(&key, &v),
                None => self.remove_extra(&key),
            }
        }
    }

    /// The keys overridden for this session.
    #[must_use]
    pub fn overlaid_keys(&self) -> Vec<String> {
        self.session_overlay.keys().cloned().collect()
    }

    /// Removes an extra key (a typed field is reset by the registry writing its default).
    pub fn remove_extra(&mut self, key: &str) {
        self.extra.remove(key);
    }
}

#[cfg(test)]
mod tests;
