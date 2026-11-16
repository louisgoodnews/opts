//! Language packs: discovery in a folder, override of a built-in language, per-key fallback to
//! English, tolerance of broken files, reload, the language list and the control methods.
//!
//! Packs are written below `~/.local/share/opts/benchmark/langpacks/` (never `/tmp`, never the
//! user's own folders); every test cleans its folder.

use std::path::{Path, PathBuf};

use bevy::prelude::*;
use opts_bevy::control::state::{ControlRegistry, Outcome};
use opts_game::language_packs;
use opts_game::settings::GameSettings;
use opts_game::settings_registry::{SettingChanged, SettingsCapabilities, SettingsRegistry};
use opts_gui::l10n::Localization;
use opts_l10n::pack::EXPECTED_LICENSE;
use serde_json::{json, Value};

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::var_os("HOME")
        .map(PathBuf::from)
        .expect("HOME")
        .join(".local/share/opts/benchmark/langpacks/tests")
        .join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

fn meta(code: &str, name: &str, native: &str, extra: &str) -> String {
    format!(
        "{{\"code\":\"{code}\",\"name\":\"{name}\",\"native_name\":\"{native}\",\"version\":\"1\",\"authors\":[\"Tester\"],\"license\":\"{EXPECTED_LICENSE}\"{extra}}}"
    )
}

fn write(dir: &Path, rel: &str, text: &str) {
    let p = dir.join(rel);
    std::fs::create_dir_all(p.parent().expect("parent")).expect("mkdir");
    std::fs::write(p, text).expect("write");
}

/// A localisation with every table the application loads.
fn full_localization() -> Localization {
    let mut loc = Localization::new("en");
    opts_game::settings_registry::add_messages(&mut loc);
    opts_game::hud::add_messages(&mut loc);
    loc
}

const CLEAR: &str = "widget-clear";
const SETTINGS: &str = "menu-settings";
#[path = "language_packs/packs.rs"]
mod packs;
#[path = "language_packs/reload.rs"]
mod reload;
#[path = "language_packs/control.rs"]
mod control;
