//! Tests of the registry core: defaults, validation, legacy bridge, migration, persistence of a
//! fixture setting declared with one descriptor, messages and the generated reference.

use super::*;

use crate::settings::GameSettings;
use opts_gui::l10n::Localization;
use rusqlite::Connection;
use std::path::PathBuf;

/// A setting declared with exactly one descriptor (the acceptance criterion of M-0063).
const FIXTURE: SettingDescriptor = SettingDescriptor::new(
    "fixture.wheel_sensitivity",
    SettingKind::Int {
        min: 1,
        max: 10,
        step: 1,
    },
    "5",
    Category::Controls,
    5,
    "set-fixture",
    "set-fixture-desc",
)
.keywords(&["steering"]);

fn caps() -> SettingsCapabilities {
    SettingsCapabilities::default()
}

fn registry() -> SettingsRegistry {
    SettingsRegistry::with_builtin()
}

#[test]
fn defaults_of_legacy_keys_equal_the_typed_defaults() {
    let r = registry();
    let s = GameSettings::default();
    for key in GameSettings::FIELD_KEYS {
        let d = r
            .get(key)
            .unwrap_or_else(|| panic!("{key} is not registered"));
        assert_eq!(
            s.raw(key).unwrap_or_default(),
            d.default,
            "default of {key} differs from the field"
        );
    }
}

#[test]
fn every_enum_option_round_trips_through_the_loader_for_legacy_keys() {
    let r = registry();
    for key in GameSettings::FIELD_KEYS {
        let d = r.get(key).expect("registered");
        if let SettingKind::Enum(options) = d.kind {
            for o in options {
                let mut s = GameSettings::default();
                s.set_raw(key, o.value);
                assert_eq!(s.raw(key).as_deref(), Some(o.value), "{key}={}", o.value);
            }
        }
    }
}

mod descriptors;
mod storage;
mod validation;

mod lighting_quality;
mod live_effects;
