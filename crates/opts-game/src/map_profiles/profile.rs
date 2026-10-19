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

/// Identity of a profile: the map and the Chrono entry its date resolves to.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MapKey {
    /// The map folder name, made filesystem-safe.
    pub map_id: String,
    /// `default` or the Chrono entry (its path below `Chrono/`), made filesystem-safe.
    pub chrono_id: String,
}

impl MapKey {
    /// The key of a map folder name and the Chrono entries active on the session date
    /// (`rel` paths below `Chrono/`; empty when none is active).
    #[must_use]
    pub fn new(map_folder: &str, active_chrono: &[&str]) -> Self {
        Self {
            map_id: slug(map_folder),
            chrono_id: chrono_id(active_chrono),
        }
    }

    /// The same map with no Chrono entry: the fallback of every Chrono key.
    #[must_use]
    pub fn map_default(&self) -> Self {
        Self {
            map_id: self.map_id.clone(),
            chrono_id: DEFAULT_CHRONO.to_string(),
        }
    }
}

/// Makes a name safe for a file name: letters, digits and `._- ` stay, everything else becomes
/// `%XX` per byte (so two different names never collide and the name stays readable).
#[must_use]
pub fn slug(name: &str) -> String {
    let mut out = String::new();
    for b in name.trim().bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-' | b' ') {
            out.push(char::from(b));
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    // A leading dot would hide the folder, and "." / ".." are not names.
    if out.is_empty() || out.starts_with('.') {
        out.insert(0, '_');
    }
    out
}

/// The Chrono part of a key: `default` without an active entry, else the entries' paths sorted
/// and joined (nesting becomes `__`), cut and hashed when very long.
#[must_use]
pub fn chrono_id(active: &[&str]) -> String {
    if active.is_empty() {
        return DEFAULT_CHRONO.to_string();
    }
    let mut parts: Vec<String> = active.iter().map(|r| slug(&r.replace('/', "__"))).collect();
    parts.sort();
    let joined = parts.join("+");
    if joined.len() <= 80 {
        return joined;
    }
    format!("{}~{:08x}", &joined[..60], fnv32(joined.as_bytes()))
}

fn fnv32(bytes: &[u8]) -> u32 {
    let mut h: u32 = 0x811c_9dc5;
    for b in bytes {
        h ^= u32::from(*b);
        h = h.wrapping_mul(0x0100_0193);
    }
    h
}

/// A stable hash of the map-eligible values (the `settings_hash` of the metrics).
#[must_use]
pub fn settings_hash(values: &BTreeMap<String, String>) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for (k, v) in values {
        for b in k.bytes().chain([b'=']).chain(v.bytes()).chain([b';']) {
            h ^= u64::from(b);
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
    }
    format!("{h:016x}")
}

/// The built-in presets (not files): *Performance*, *Balanced*, *Quality*.
#[must_use]
pub fn presets() -> Vec<MapProfile> {
    let make = |id: &str, name: &str, pairs: &[(&str, &str)]| {
        let mut p = MapProfile::named(name);
        p.created = String::new();
        p.based_on = Some(id.to_string());
        p.settings = pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect();
        p
    };
    vec![
        make(
            "performance",
            "Performance",
            &[
                ("graphics.light_quality_mode", "custom"),
                ("graphics.light_shafts", "off"),
                ("graphics.light_night_shafts", "false"),
                ("graphics.light_cone_cap", "24"),
                ("graphics.light_cone_depth_fade", "false"),
                ("graphics.mirror_count", "1"),
                ("graphics.mirror_rate", "4"),
                ("graphics.glass_reflections", "false"),
                ("graphics.bloom", "false"),
                ("graphics.weather_precipitation", "low"),
                ("graphics.weather_wet_ground", "false"),
                ("texture_filtering", "4"),
                ("gameplay.traffic", "40"),
                ("gameplay.pedestrians", "10"),
                ("gameplay.tile_radius", "2"),
            ],
        ),
        make(
            "balanced",
            "Balanced",
            &[
                ("graphics.light_quality_mode", "automatic"),
                ("graphics.mirror_count", "2"),
                ("graphics.mirror_rate", "2"),
                ("graphics.weather_precipitation", "medium"),
                ("texture_filtering", "8"),
                ("gameplay.traffic", "80"),
                ("gameplay.pedestrians", "30"),
                ("gameplay.tile_radius", "0"),
            ],
        ),
        make(
            "quality",
            "Quality",
            &[
                ("graphics.light_quality_mode", "custom"),
                ("graphics.light_shafts", "high"),
                ("graphics.light_night_shafts", "true"),
                ("graphics.light_cone_cap", "96"),
                ("graphics.light_cone_depth_fade", "true"),
                ("graphics.mirror_count", "2"),
                ("graphics.mirror_rate", "1"),
                ("graphics.glass_reflections", "true"),
                ("graphics.weather_precipitation", "high"),
                ("graphics.weather_wet_ground", "true"),
                ("texture_filtering", "16"),
                ("gameplay.traffic", "120"),
                ("gameplay.pedestrians", "60"),
            ],
        ),
    ]
}
