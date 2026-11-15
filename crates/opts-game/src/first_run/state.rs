//! The state of the first-run flow: the open step, the refused folder and the welcome's memory.

use std::path::PathBuf;

use bevy::prelude::*;

use crate::settings::GameSettings;

use super::ChooseError;

/// The step the first-run flow shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step {
    /// Nothing was found: choose a folder.
    Choose,
    /// The content was read: welcome.
    Welcome,
}

/// State of the flow.
#[derive(Resource, Debug)]
pub struct FirstRun {
    /// The open step, if any.
    pub step: Option<Step>,
    /// Why the last chosen folder was refused (shown inline in the dialog).
    pub error: Option<ChooseError>,
    /// The welcome was shown (or is not needed): it never comes back.
    pub welcome_done: bool,
    /// Whether `welcome_done` is read from and written to the user's data directory.
    pub persist: bool,
}

impl Default for FirstRun {
    fn default() -> Self {
        Self {
            step: None,
            error: None,
            welcome_done: false,
            persist: true,
        }
    }
}

pub(super) fn state_file() -> Option<PathBuf> {
    GameSettings::database_path().map(|p| p.with_file_name("first_run.json"))
}

impl FirstRun {
    /// The remembered state: whether the welcome was shown.
    pub fn load() -> Self {
        let done = state_file()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
            .and_then(|v| v.get("welcome_done").and_then(serde_json::Value::as_bool))
            .unwrap_or(false);
        Self {
            welcome_done: done,
            ..Self::default()
        }
    }

    pub(super) fn finish_welcome(&mut self) {
        self.step = None;
        self.welcome_done = true;
        if !self.persist {
            return;
        }
        if let Some(path) = state_file() {
            if let Some(dir) = path.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            if let Err(e) = std::fs::write(path, r#"{"welcome_done":true}"#) {
                tracing::warn!("could not remember that the welcome was shown: {e}");
            }
        }
    }
}
