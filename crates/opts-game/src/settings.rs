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

mod load;
mod session;

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


#[cfg(test)]
mod tests;
