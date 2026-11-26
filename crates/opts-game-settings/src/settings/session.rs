//! The settings' keys by name: the typed fields and the extra keys, the session overlay of a map
//! profile, and the removal of an extra key.

use std::collections::BTreeMap;
use std::path::PathBuf;

use super::GameSettings;

impl GameSettings {
    /// The keys that have a typed field of their own; all others live in [`Self::extra`].
    pub const FIELD_KEYS: [&'static str; 14] = [
        "recent_map_path",
        "omsi_path",
        "discovery_interval_days",
        "texture_filtering",
        "look",
        "theme_mode",
        "auto_light_from",
        "auto_dark_from",
        "high_contrast",
        "active_theme",
        "theme_file",
        "ui_scale",
        "density",
        "reduced_motion",
    ];

    /// Applies one stored key (the loader's parser): a typed field, or [`Self::extra`] for any
    /// other key. Unreadable values of a field leave its default.
    pub fn apply_key(&mut self, key: &str, value: &str) {
        match key {
            "recent_map_path" => {
                self.recent_map_path = (!value.trim().is_empty()).then(|| PathBuf::from(value));
            }
            "omsi_path" => {
                self.omsi_path = (!value.trim().is_empty()).then(|| PathBuf::from(value));
            }
            "discovery_interval_days" => {
                self.discovery_interval_days = value.parse::<u32>().unwrap_or(1);
            }
            "texture_filtering" => {
                self.texture_filtering =
                    opts_bevy::TextureFiltering::parse(value).unwrap_or_default();
            }
            "look" => {
                self.look = crate::look::Look::parse(value).unwrap_or_default();
            }
            other => {
                if !self.apply_theme_key(other, value) {
                    self.extra.insert(other.to_string(), value.to_string());
                }
            }
        }
    }

    /// The stored text of `key` (a field or an extra key), `None` when nothing is stored.
    #[must_use]
    pub fn raw(&self, key: &str) -> Option<String> {
        if Self::FIELD_KEYS.contains(&key) {
            self.entries().get(key).cloned()
        } else {
            self.extra.get(key).cloned()
        }
    }

    /// Stores `value` under `key` without validation (see the registry for validated writes).
    pub fn set_raw(&mut self, key: &str, value: &str) {
        self.apply_key(key, value);
    }

    /// Overrides keys for the running session (a map profile): the values are set in memory, the
    /// global values are remembered, and [`save`](Self::save) keeps writing the global ones.
    /// Applying again first restores what an earlier call overrode.
    pub fn overlay_session(&mut self, values: &BTreeMap<String, String>) {
        self.restore_session();
        for (key, value) in values {
            self.session_overlay.insert(key.clone(), self.raw(key));
            self.set_raw(key, value);
        }
    }

    /// Puts the global values back after [`overlay_session`](Self::overlay_session).
    pub fn restore_session(&mut self) {
        for (key, global) in std::mem::take(&mut self.session_overlay) {
            match global {
                Some(v) => self.set_raw(&key, &v),
                None => self.remove_extra(&key),
            }
        }
    }

    /// The keys overridden for this session.
    #[must_use]
    pub fn overlaid_keys(&self) -> Vec<String> {
        self.session_overlay.keys().cloned().collect()
    }

    /// Removes an extra key (a typed field is reset by the registry writing its default).
    pub fn remove_extra(&mut self, key: &str) {
        self.extra.remove(key);
    }
}
