//! Loading the settings: the immutable reference file, the database's tables, and the stored
//! keys read back into the settings.

use std::fs;
use std::path::PathBuf;

use directories::ProjectDirs;
use rusqlite::Connection;

use super::GameSettings;

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
}
