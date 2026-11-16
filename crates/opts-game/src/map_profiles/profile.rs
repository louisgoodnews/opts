//! The profile itself: a named set of map-eligible settings, its key, the built-in presets and
//! the TOML text (`docs/src/architecture/map-profiles.md` §1 and §2).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::settings_registry::{normalize, ProfileScope, SettingsRegistry};

/// Schema version written to every profile file.
pub const SCHEMA: u32 = 1;
/// The Chrono part of a key when no Chrono entry covers the date.
pub const DEFAULT_CHRONO: &str = "default";

/// One saved profile: only keys whose descriptor says [`ProfileScope::Map`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapProfile {
    /// File schema (`1`).
    #[serde(default = "schema_one")]
    pub schema: u32,
    /// Name shown in the dropdown.
    #[serde(default)]
    pub name: String,
    /// RFC 3339 time of creation (informational).
    #[serde(default)]
    pub created: String,
    /// The preset it was created from, if any.
    #[serde(default)]
    pub based_on: Option<String>,
    /// Key to stored text, as the registry writes values.
    #[serde(default)]
    pub settings: BTreeMap<String, String>,
}

fn schema_one() -> u32 {
    SCHEMA
}

/// A key of a profile left out while loading, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dropped {
    /// The setting key as written in the file.
    pub key: String,
    /// What was wrong with it.
    pub reason: String,
}

/// Why a profile file could not be used at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfileError {
    /// The text is not valid TOML of the profile shape.
    Syntax(String),
}

impl std::fmt::Display for ProfileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Syntax(m) => write!(f, "not a profile file: {m}"),
        }
    }
}

impl MapProfile {
    /// An empty profile with a name, stamped with the current time.
    #[must_use]
    pub fn named(name: &str) -> Self {
        Self {
            schema: SCHEMA,
            name: name.to_string(),
            created: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            based_on: None,
            settings: BTreeMap::new(),
        }
    }

    /// Reads a profile and validates every value against the registry: a key that is unknown, is
    /// not map-profile eligible any more, or holds an invalid value is dropped (and reported),
    /// the rest loads. A `schema` newer than this build's is read as far as it is understood.
    ///
    /// # Errors
    /// The text is not TOML of the profile shape.
    pub fn parse(
        text: &str,
        registry: &SettingsRegistry,
    ) -> Result<(Self, Vec<Dropped>), ProfileError> {
        let mut p: Self = toml::from_str(text).map_err(|e| ProfileError::Syntax(e.to_string()))?;
        let mut dropped = Vec::new();
        let mut kept = BTreeMap::new();
        for (key, value) in std::mem::take(&mut p.settings) {
            let Some(d) = registry.get(&key) else {
                dropped.push(Dropped {
                    key,
                    reason: "unknown setting".into(),
                });
                continue;
            };
            if d.profile != ProfileScope::Map {
                dropped.push(Dropped {
                    key,
                    reason: "not a map-profile setting".into(),
                });
                continue;
            }
            match normalize(d, &value) {
                Ok(v) => {
                    kept.insert(d.key.to_string(), v);
                }
                Err(e) => dropped.push(Dropped {
                    key,
                    reason: e.to_string(),
                }),
            }
        }
        p.settings = kept;
        Ok((p, dropped))
    }

    /// The TOML text of the profile.
    #[must_use]
    pub fn to_toml(&self) -> String {
        toml::to_string_pretty(self).unwrap_or_default()
    }

    /// A profile of the map-eligible keys with the values `current` holds (`Create New from
    /// Current`); keys `current` does not know are left out.
    #[must_use]
    pub fn from_current(
        name: &str,
        registry: &SettingsRegistry,
        current: impl Fn(&str) -> Option<String>,
    ) -> Self {
        let mut p = Self::named(name);
        for key in registry.profile_keys() {
            if let Some(v) = current(key) {
                p.settings.insert(key.to_string(), v);
            }
        }
        p
    }
}

mod builtin;
mod ids;
pub use builtin::presets;
pub use ids::{chrono_id, settings_hash, slug, MapKey};
