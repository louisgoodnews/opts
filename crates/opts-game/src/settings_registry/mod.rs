//! The settings registry: one declaration per setting (see
//! `docs/src/architecture/settings-registry.md`).
//!
//! A feature registers a [`SettingDescriptor`] with [`SettingsAppExt::register_setting`]; the
//! settings page, persistence (`settings.db`), validation, *Reset*, search, the control-protocol
//! commands `settings.*` and the generated reference follow from it. Values are stored as text
//! in the key/value table, the descriptor is the schema. The typed fields of
//! [`GameSettings`] stay for the existing consumers; the registry reaches them through the
//! loader's own parser ([`GameSettings::raw`] / [`GameSettings::set_raw`]).

use std::collections::{BTreeMap, BTreeSet};

use bevy::prelude::*;
use opts_gui::l10n::Localization;

pub mod api;
pub mod builtin;
pub mod descriptor;
pub mod display;
pub mod live;
pub mod protocol;
pub mod reference;
pub mod search;
pub mod value;

pub use api::SettingsApi;
pub use descriptor::{opt, Apply, Available, Category, EnumOption, SettingDescriptor, SettingKind};
pub use value::{normalize, SettingError, SettingValue};

use crate::settings::GameSettings;

/// Where a change came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeSource {
    /// The settings page.
    Page,
    /// The control protocol (`settings.set`).
    Protocol,
    /// *Reset* of a row.
    Reset,
    /// The revert prompt gave the old value back.
    Revert,
}

/// Message written after a setting changed (and was saved).
#[derive(Message, Debug, Clone, PartialEq, Eq)]
pub struct SettingChanged {
    /// The key.
    pub key: String,
    /// Stored text before.
    pub old: String,
    /// Stored text now.
    pub new: String,
    /// Where it came from.
    pub source: ChangeSource,
}

/// What the machine and the build can do; consulted by [`Available::Capability`].
#[derive(Resource, Debug, Clone, Default)]
pub struct SettingsCapabilities(pub BTreeSet<String>);

impl SettingsCapabilities {
    /// Whether `name` is present.
    #[must_use]
    pub fn has(&self, name: &str) -> bool {
        self.0.contains(name)
    }
}

/// All registered descriptors.
#[derive(Resource, Debug, Clone, Default)]
pub struct SettingsRegistry {
    by_key: BTreeMap<&'static str, SettingDescriptor>,
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

/// Registration on the [`App`].
pub trait SettingsAppExt {
    /// Registers one setting.
    fn register_setting(&mut self, d: SettingDescriptor) -> &mut Self;
    /// Registers several settings.
    fn register_settings(&mut self, ds: &[SettingDescriptor]) -> &mut Self;
}

impl SettingsAppExt for App {
    fn register_setting(&mut self, d: SettingDescriptor) -> &mut Self {
        self.init_resource::<SettingsRegistry>();
        self.world_mut()
            .resource_mut::<SettingsRegistry>()
            .register(d);
        self
    }

    fn register_settings(&mut self, ds: &[SettingDescriptor]) -> &mut Self {
        for d in ds {
            self.register_setting(*d);
        }
        self
    }
}

/// Changes settings from outside a system (the control protocol, tests): validates, stores,
/// saves, announces.
pub fn apply_change(
    world: &mut World,
    key: &str,
    text: Option<&str>,
    source: ChangeSource,
) -> Result<SettingChanged, SettingError> {
    let registry = world.resource::<SettingsRegistry>().clone();
    let caps = world
        .get_resource::<SettingsCapabilities>()
        .cloned()
        .unwrap_or_default();
    let mut settings = world.resource_mut::<GameSettings>();
    let changed = api::change_in(&registry, &caps, &mut settings, key, text, source, true)?;
    if changed.old != changed.new {
        world.write_message(changed.clone());
    }
    Ok(changed)
}

/// A [`Localization`] for `locale` that also knows the messages of the settings (tests and the
/// generated reference).
#[must_use]
pub fn localization(locale: &str) -> Localization {
    let mut loc = Localization::new(locale);
    add_messages(&mut loc);
    loc
}

/// Adds the settings messages (`en`, `de`) to `loc`.
pub fn add_messages(loc: &mut Localization) {
    loc.extend("en", include_str!("messages_en.ftl"));
    loc.extend("de", include_str!("messages_de.ftl"));
}

/// Adds the registry, the message and the migration of renamed keys.
pub struct SettingsRegistryPlugin;

impl Plugin for SettingsRegistryPlugin {
    fn build(&self, app: &mut App) {
        if !app.world().contains_resource::<SettingsRegistry>() {
            app.insert_resource(SettingsRegistry::with_builtin());
        } else {
            let mut r = app.world_mut().resource_mut::<SettingsRegistry>();
            for d in builtin::all() {
                r.register(d);
            }
        }
        if !app.world().contains_resource::<Localization>() {
            app.insert_resource(Localization::detect());
        }
        add_messages(&mut app.world_mut().resource_mut::<Localization>());
        app.init_resource::<SettingsCapabilities>()
            .add_message::<SettingChanged>()
            .insert_resource(display::DisplayBackend(Box::new(display::WindowDisplay)))
            .add_systems(
                Startup,
                (migrate_registered, display::apply_saved_display).chain(),
            )
            .add_systems(
                Update,
                (
                    display::apply_display_changes,
                    live::apply_live_changes,
                    live::sync_developer_panel,
                    live::sync_water,
                    live::sync_weather_effects,
                ),
            )
            .add_systems(
                Startup,
                (
                    crate::language_packs::load_startup,
                    live::apply_saved_language,
                    live::apply_saved_trace,
                )
                    .chain(),
            )
            .add_systems(Update, crate::language_packs::reload_action);
        protocol::register(app);
    }
}

fn migrate_registered(registry: Res<SettingsRegistry>, mut settings: ResMut<GameSettings>) {
    let done = registry.migrate(&mut settings);
    if !done.is_empty() {
        tracing::info!(keys = ?done, "migrated renamed settings");
    }
}

#[cfg(test)]
mod tests;
