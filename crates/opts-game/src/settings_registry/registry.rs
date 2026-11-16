//! The registry's lookups: its descriptors, the stored text and the typed value of a setting.

use std::collections::BTreeMap;

use bevy::prelude::*;

use super::{
    builtin, normalize, value, Available, Category, ProfileScope, SettingDescriptor, SettingError,
    SettingValue, SettingsCapabilities,
};
use crate::settings::GameSettings;

/// All registered descriptors.
#[derive(Resource, Debug, Clone, Default)]
pub struct SettingsRegistry {
    pub(super) by_key: BTreeMap<&'static str, SettingDescriptor>,
}

impl SettingsRegistry {
    /// An empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registry with the built-in settings of the game.
    #[must_use]
    pub fn with_builtin() -> Self {
        let mut r = Self::new();
        for d in builtin::all() {
            r.register(d);
        }
        r
    }

    /// Adds a descriptor. A key registered twice keeps the first and logs a warning.
    pub fn register(&mut self, d: SettingDescriptor) {
        if self.by_key.contains_key(d.key) {
            tracing::warn!(key = d.key, "setting registered twice, keeping the first");
            return;
        }
        self.by_key.insert(d.key, d);
    }

    /// The keys a map profile may set, in key order.
    #[must_use]
    pub fn profile_keys(&self) -> Vec<&'static str> {
        self.by_key
            .values()
            .filter(|d| d.profile == ProfileScope::Map)
            .map(|d| d.key)
            .collect()
    }

    /// The descriptor of `key`.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&SettingDescriptor> {
        self.by_key.get(key)
    }

    /// Every descriptor, ordered by category, order, key.
    #[must_use]
    pub fn all(&self) -> Vec<&SettingDescriptor> {
        let mut v: Vec<&SettingDescriptor> = self.by_key.values().collect();
        v.sort_by_key(|d| (d.category, d.order, d.key));
        v
    }

    /// The descriptors of one category, in order.
    #[must_use]
    pub fn in_category(&self, c: Category) -> Vec<&SettingDescriptor> {
        self.all().into_iter().filter(|d| d.category == c).collect()
    }

    /// The categories that have at least one setting.
    #[must_use]
    pub fn categories(&self) -> Vec<Category> {
        Category::ALL
            .into_iter()
            .filter(|c| self.by_key.values().any(|d| d.category == *c))
            .collect()
    }

    /// The current stored text of `key`: the stored value if it is valid, else the default.
    #[must_use]
    pub fn text(&self, settings: &GameSettings, key: &str) -> Option<String> {
        let d = self.get(key)?;
        Some(match settings.raw(key) {
            Some(raw) => normalize(d, &raw).unwrap_or_else(|_| d.default.to_string()),
            None => d.default.to_string(),
        })
    }

    /// The typed current value of `key`.
    #[must_use]
    pub fn value(&self, settings: &GameSettings, key: &str) -> Option<SettingValue> {
        let d = self.get(key)?;
        Some(value::parse(d, &self.text(settings, key)?))
    }

    /// Whether the descriptor can be changed now (not planned, capability present).
    pub fn check_available(
        &self,
        d: &SettingDescriptor,
        caps: &SettingsCapabilities,
    ) -> Result<(), SettingError> {
        match d.available {
            Available::Always => Ok(()),
            Available::Capability(c) if caps.has(c) => Ok(()),
            Available::Capability(c) => Err(SettingError::Unavailable(format!(
                "{}: needs {c}, which this machine or build does not have",
                d.key
            ))),
            Available::NotYet(p) => Err(SettingError::Unavailable(format!(
                "{}: Feature not available yet! ({})",
                d.key, p.milestone
            ))),
        }
    }
}
