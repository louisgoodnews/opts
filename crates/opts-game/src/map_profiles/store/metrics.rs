//! Session metrics of a map: the record of each drive, the last session, and the metrics file beside the profiles.

use super::*;

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

impl ProfileStore {
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
