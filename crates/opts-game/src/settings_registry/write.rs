//! Changing the registry's values: validated set, reset to the default and the migration of
//! renamed keys.

use super::{SettingError, SettingKind, SettingsCapabilities, SettingsRegistry, normalize};
use crate::settings::GameSettings;

impl SettingsRegistry {
    /// Validates and stores `text` (no save, no message). Returns the old and new stored text.
    pub fn set(
        &self,
        settings: &mut GameSettings,
        caps: &SettingsCapabilities,
        key: &str,
        text: &str,
    ) -> Result<(String, String), SettingError> {
        let d = self
            .get(key)
            .ok_or_else(|| SettingError::Unknown(key.to_string()))?;
        self.check_available(d, caps)?;
        let new = normalize(d, text)?;
        if matches!(d.kind, SettingKind::Binding) {
            if let Ok(Some(chord)) = opts_bevy::KeyChord::parse(&new) {
                if let Some(why) = crate::action_settings::conflict(self, settings, key, chord) {
                    return Err(SettingError::Invalid(format!("{key}: {why}")));
                }
            }
        }
        let old = self.text(settings, key).unwrap_or_default();
        settings.set_raw(key, &new);
        Ok((old, new))
    }

    /// Restores the default of `key`; extra keys are removed from the file's perspective by
    /// writing the default (so the baseline diff stays simple).
    pub fn reset(
        &self,
        settings: &mut GameSettings,
        caps: &SettingsCapabilities,
        key: &str,
    ) -> Result<(String, String), SettingError> {
        let d = self
            .get(key)
            .ok_or_else(|| SettingError::Unknown(key.to_string()))?;
        self.set(settings, caps, key, d.default)
    }

    /// `migrate_from`: for every setting whose key was not loaded, takes the first older key that
    /// was, when its value is valid. Returns the migrated keys.
    pub fn migrate(&self, settings: &mut GameSettings) -> Vec<&'static str> {
        let mut done = Vec::new();
        for d in self.by_key.values() {
            if d.migrate_from.is_empty() || settings.loaded_keys.contains(d.key) {
                continue;
            }
            for old in d.migrate_from {
                if !settings.loaded_keys.contains(*old) {
                    continue;
                }
                let Some(raw) = settings
                    .extra
                    .get(*old)
                    .cloned()
                    .or_else(|| settings.raw(old))
                else {
                    continue;
                };
                if let Ok(v) = normalize(d, &raw) {
                    settings.set_raw(d.key, &v);
                    done.push(d.key);
                    break;
                }
            }
        }
        done
    }
}
