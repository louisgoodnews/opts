//! The profile files: `<root>/<map_id>/<chrono_id>.toml`, the fallback chain and the metrics file
//! (`docs/src/architecture/map-profiles.md` §2 and §3.1).

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::profile::{Dropped, MapKey, MapProfile, DEFAULT_CHRONO};
use crate::settings_registry::SettingsRegistry;

/// Sessions kept per Chrono key in `metrics.toml`.
pub const SESSIONS_KEPT: usize = 5;
/// Sessions shorter than this (seconds) are not recorded.
pub const MIN_SESSION_S: f32 = 60.0;

/// Where a resolved profile came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileSource {
    /// The file of this map and Chrono entry.
    Chrono,
    /// The map's `default.toml`.
    Map,
    /// No profile: the global settings apply.
    Global,
}

/// The outcome of the fallback chain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolved {
    /// The profile to apply, none for [`ProfileSource::Global`].
    pub profile: Option<MapProfile>,
    /// Which link of the chain gave it.
    pub source: ProfileSource,
    /// Keys left out while loading.
    pub dropped: Vec<Dropped>,
}

/// One played session of a map, as `metrics.toml` keeps it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionMetrics {
    /// The Chrono part of the key the session ran under.
    pub chrono: String,
    /// Average frame time (ms).
    pub avg_ms: f32,
    /// 95th-percentile frame time (ms).
    pub p95_ms: f32,
    /// Seconds played.
    pub seconds: f32,
    /// The vehicle (display name, empty without one).
    #[serde(default)]
    pub vehicle: String,
    /// Hash of the map-eligible values of the session ([`super::profile::settings_hash`]).
    #[serde(default)]
    pub settings_hash: String,
    /// RFC 3339 date.
    #[serde(default)]
    pub date: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct MetricsFile {
    #[serde(default)]
    sessions: Vec<SessionMetrics>,
}

/// The profile folder.
#[derive(Debug, Clone)]
pub struct ProfileStore {
    root: PathBuf,
}

impl ProfileStore {
    /// A store rooted at `root` (created on the first write).
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// `<user data dir>/profiles`, beside `drive_history.json`.
    #[must_use]
    pub fn default_root() -> Option<PathBuf> {
        crate::drive_history::DriveHistory::path()
            .and_then(|p| p.parent().map(|d| d.join("profiles")))
    }

    /// The store of the user's data directory.
    #[must_use]
    pub fn user() -> Option<Self> {
        Self::default_root().map(Self::new)
    }

    /// The folder of the store.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    fn map_dir(&self, map_id: &str) -> PathBuf {
        self.root.join(map_id)
    }

    fn file(&self, key: &MapKey) -> PathBuf {
        self.map_dir(&key.map_id)
            .join(format!("{}.toml", key.chrono_id))
    }

    /// Reads the profile of exactly this key. A file that does not parse is moved aside as
    /// `<name>.toml.bad` and counts as absent.
    #[must_use]
    pub fn load(
        &self,
        registry: &SettingsRegistry,
        key: &MapKey,
    ) -> Option<(MapProfile, Vec<Dropped>)> {
        let path = self.file(key);
        let text = fs::read_to_string(&path).ok()?;
        match MapProfile::parse(&text, registry) {
            Ok(ok) => Some(ok),
            Err(e) => {
                tracing::warn!(path = %path.display(), "{e}; moved aside");
                let mut bad = path.clone().into_os_string();
                bad.push(".bad");
                let _ = fs::rename(&path, bad);
                None
            }
        }
    }

    /// The fallback chain: this Chrono entry, then the map's default, then nothing (global).
    #[must_use]
    pub fn resolve(&self, registry: &SettingsRegistry, key: &MapKey) -> Resolved {
        if key.chrono_id != DEFAULT_CHRONO {
            if let Some((profile, dropped)) = self.load(registry, key) {
                return Resolved {
                    profile: Some(profile),
                    source: ProfileSource::Chrono,
                    dropped,
                };
            }
        }
        if let Some((profile, dropped)) = self.load(registry, &key.map_default()) {
            return Resolved {
                profile: Some(profile),
                source: ProfileSource::Map,
                dropped,
            };
        }
        Resolved {
            profile: None,
            source: ProfileSource::Global,
            dropped: Vec::new(),
        }
    }

    /// Writes the profile of `key` (atomic: a temporary file, then a rename).
    ///
    /// # Errors
    /// The folder or file cannot be written.
    pub fn save(&self, key: &MapKey, profile: &MapProfile) -> io::Result<()> {
        write_atomic(&self.file(key), &profile.to_toml())
    }

    /// Deletes the profile of `key`; `false` when there was none.
    ///
    /// # Errors
    /// The file exists but cannot be removed.
    pub fn delete(&self, key: &MapKey) -> io::Result<bool> {
        match fs::remove_file(self.file(key)) {
            Ok(()) => Ok(true),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(e),
        }
    }

    /// Every profile saved for the map: `(chrono_id, profile)` ordered by chrono id.
    #[must_use]
    pub fn list_map(&self, registry: &SettingsRegistry, map_id: &str) -> Vec<(String, MapProfile)> {
        let mut out = Vec::new();
        let Ok(rd) = fs::read_dir(self.map_dir(map_id)) else {
            return out;
        };
        for e in rd.flatten() {
            let path = e.path();
            let (Some(stem), Some("toml")) = (
                path.file_stem().and_then(|s| s.to_str()),
                path.extension().and_then(|s| s.to_str()),
            ) else {
                continue;
            };
            // The two other files of a map folder are not profiles.
            if stem == "metrics" || stem == "static" {
                continue;
            }
            let key = MapKey {
                map_id: map_id.to_string(),
                chrono_id: stem.to_string(),
            };
            if let Some((p, _)) = self.load(registry, &key) {
                out.push((stem.to_string(), p));
            }
        }
        out.sort_by(|a, b| a.0.cmp(&b.0));
        out
    }

    fn metrics_path(&self, map_id: &str) -> PathBuf {
        self.map_dir(map_id).join("metrics.toml")
    }

    /// The recorded sessions of the map, oldest first (an unreadable file counts as empty).
    #[must_use]
    pub fn sessions(&self, map_id: &str) -> Vec<SessionMetrics> {
        fs::read_to_string(self.metrics_path(map_id))
            .ok()
            .and_then(|t| toml::from_str::<MetricsFile>(&t).ok())
            .map(|m| m.sessions)
            .unwrap_or_default()
    }

    /// The newest recorded session under the Chrono key of `key`.
    #[must_use]
    pub fn last_session(&self, key: &MapKey) -> Option<SessionMetrics> {
        self.sessions(&key.map_id)
            .into_iter()
            .rev()
            .find(|s| s.chrono == key.chrono_id)
    }

    /// Records a session: sessions under [`MIN_SESSION_S`] are dropped, and only the newest
    /// [`SESSIONS_KEPT`] per Chrono key stay. Returns whether it was recorded.
    ///
    /// # Errors
    /// The file cannot be written.
    pub fn record_session(&self, map_id: &str, session: SessionMetrics) -> io::Result<bool> {
        if session.seconds < MIN_SESSION_S || !session.avg_ms.is_finite() {
            return Ok(false);
        }
        let mut all = self.sessions(map_id);
        all.push(session.clone());
        let same: Vec<usize> = all
            .iter()
            .enumerate()
            .filter(|(_, s)| s.chrono == session.chrono)
            .map(|(i, _)| i)
            .collect();
        if same.len() > SESSIONS_KEPT {
            let drop: std::collections::HashSet<usize> = same
                .iter()
                .copied()
                .take(same.len() - SESSIONS_KEPT)
                .collect();
            let mut i = 0;
            all.retain(|_| {
                let keep = !drop.contains(&i);
                i += 1;
                keep
            });
        }
        let text = toml::to_string_pretty(&MetricsFile { sessions: all }).unwrap_or_default();
        write_atomic(&self.metrics_path(map_id), &text)?;
        Ok(true)
    }
}

fn write_atomic(path: &Path, text: &str) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = PathBuf::from(tmp);
    fs::write(&tmp, text)?;
    fs::rename(&tmp, path)
}
