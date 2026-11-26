//! The persisted key/value pairs of the settings, and the baseline they are compared with.

use std::collections::BTreeMap;
use std::path::PathBuf;

use super::GameSettings;

impl GameSettings {
    /// The persisted values as key/value pairs.
    pub fn entries(&self) -> BTreeMap<String, String> {
        let path = |p: &Option<PathBuf>| {
            p.as_ref()
                .and_then(|p| p.to_str())
                .unwrap_or("")
                .to_string()
        };
        let mut m = BTreeMap::new();
        m.insert("recent_map_path".into(), path(&self.recent_map_path));
        m.insert("omsi_path".into(), path(&self.omsi_path));
        m.insert(
            "discovery_interval_days".into(),
            self.discovery_interval_days.to_string(),
        );
        m.insert(
            "texture_filtering".into(),
            self.texture_filtering.key().to_string(),
        );
        m.insert("look".into(), self.look.key().to_string());
        m.extend(self.theme_entries());
        for (k, v) in &self.extra {
            m.entry(k.clone()).or_insert_with(|| v.clone());
        }
        // A profile's values are the session's, not the person's: the file keeps the global ones.
        for (k, global) in &self.session_overlay {
            match global {
                Some(v) => {
                    m.insert(k.clone(), v.clone());
                }
                None => {
                    m.remove(k);
                }
            }
        }
        m
    }

    pub fn remember_baseline(&self) {
        if let Ok(mut b) = self.baseline.0.lock() {
            *b = Some(self.entries());
        }
    }
}
