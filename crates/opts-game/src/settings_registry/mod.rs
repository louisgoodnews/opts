//! The settings registry: one declaration per setting (see
//! `docs/src/architecture/settings-registry.md`).
//!
//! A feature registers a [`SettingDescriptor`] with [`SettingsAppExt::register_setting`]; the
//! settings page, persistence (`settings.db`), validation, *Reset*, search, the control-protocol
//! commands `settings.*` and the generated reference follow from it. Values are stored as text
//! in the key/value table, the descriptor is the schema. The typed fields of
//! [`crate::settings::GameSettings`] stay for the existing consumers; the registry reaches them through the
//! loader's own parser ([`crate::settings::GameSettings::raw`] / [`crate::settings::GameSettings::set_raw`]).

use std::collections::BTreeSet;

use bevy::prelude::*;

pub mod api;
pub mod builtin;
pub mod descriptor;
pub mod display;
pub mod lighting;
pub mod live;
pub mod protocol;
pub mod protocol_batch;
pub mod protocol_confirm;
pub mod reference;
pub mod search;
pub mod value;

mod change;
mod messages;
mod plugin;
mod registry;
mod write;

pub use api::SettingsApi;
pub use change::{apply_change, apply_change_with};
pub use descriptor::{
    opt, Apply, Available, Category, EnumOption, ProfileScope, SettingDescriptor, SettingKind,
};
pub use messages::{add_messages, localization};
pub use plugin::{SettingsAppExt, SettingsRegistryPlugin};
pub use registry::SettingsRegistry;
pub use value::{normalize, SettingError, SettingValue};

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

#[cfg(test)]
mod tests;
