//! Tests of the map profiles, split by concern: profile files, metrics, advice rules and the session
//! overlay on `GameSettings`.

use std::path::PathBuf;

use super::*;

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

mod advice_rules;
mod files;
mod metrics;
mod overlay;
