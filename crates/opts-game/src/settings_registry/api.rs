//! The one validated way to change a setting from a system: [`SettingsApi`].

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use super::{ChangeSource, SettingChanged, SettingError, SettingsCapabilities, SettingsRegistry};
use crate::settings::GameSettings;

/// Validates, stores, (optionally) saves and announces a change. Shared by [`SettingsApi`] and
/// the world based [`super::apply_change`].
pub fn change_in(
    registry: &SettingsRegistry,
    caps: &SettingsCapabilities,
    settings: &mut GameSettings,
    key: &str,
    text: Option<&str>,
    source: ChangeSource,
    save: bool,
) -> Result<SettingChanged, SettingError> {
    let (old, new) = match text {
        Some(t) => registry.set(settings, caps, key, t)?,
        None => registry.reset(settings, caps, key)?,
    };
    if save && old != new {
        // Off the main thread: a click never waits for SQLite.
        settings.save_in_background();
    }
    Ok(SettingChanged {
        key: key.to_string(),
        old,
        new,
        source,
    })
}

/// Read and change settings from a system.
#[derive(SystemParam)]
pub struct SettingsApi<'w> {
    /// The descriptors.
    pub registry: Res<'w, SettingsRegistry>,
    /// What the machine can do.
    pub caps: Res<'w, SettingsCapabilities>,
    /// The values.
    pub settings: ResMut<'w, GameSettings>,
    out: MessageWriter<'w, SettingChanged>,
}

impl SettingsApi<'_> {
    /// The current stored text of `key` (the default when unset or unreadable).
    #[must_use]
    pub fn text(&self, key: &str) -> String {
        self.registry.text(&self.settings, key).unwrap_or_default()
    }

    /// Changes (`Some(text)`) or resets (`None`) a setting and announces it. With `save` false
    /// the value is applied but not written (the revert prompt saves after *Keep*).
    pub fn change(
        &mut self,
        key: &str,
        text: Option<&str>,
        source: ChangeSource,
        save: bool,
    ) -> Result<SettingChanged, SettingError> {
        let c = change_in(
            &self.registry,
            &self.caps,
            &mut self.settings,
            key,
            text,
            source,
            save,
        )?;
        if c.old != c.new {
            self.out.write(c.clone());
        }
        Ok(c)
    }

    /// Writes the settings file (after a confirmed risky change).
    pub fn save(&mut self) {
        self.settings.save_in_background();
    }
}
