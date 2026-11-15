//! Profile files, the fallback chain, validation, presets, the metrics ring, the advice rules and
//! the session overlay on `GameSettings`.

use std::collections::BTreeMap;
use std::path::PathBuf;

use super::advice::{self, tiles_to_km2};
use super::profile::{chrono_id, settings_hash, slug, Dropped};
use super::*;
use crate::settings::GameSettings;
use crate::settings_registry::SettingsRegistry;

fn dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("opts-profiles-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

fn registry() -> SettingsRegistry {
    SettingsRegistry::with_builtin()
}

fn profile(name: &str, pairs: &[(&str, &str)]) -> MapProfile {
    let mut p = MapProfile::named(name);
    p.settings = pairs
        .iter()
        .map(|(k, v)| ((*k).into(), (*v).into()))
        .collect();
    p
}

mod advice_metrics;
mod chain;
mod keys;
mod overlay;
