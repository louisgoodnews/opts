//! Unit tests for `super` (`vehicle_keys.rs`).

mod drive;
mod registry;

use super::*;
use crate::settings::GameSettings;
use crate::settings_registry::{SettingsCapabilities, SettingsRegistry};

fn parts() -> (SettingsRegistry, SettingsCapabilities, GameSettings) {
    (
        SettingsRegistry::with_builtin(),
        SettingsCapabilities::default(),
        GameSettings::default(),
    )
}
