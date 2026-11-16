//! The builder methods of [`SettingDescriptor`]: each one sets one field of a descriptor
//! declared with [`SettingDescriptor::new`].

use opts_gui::not_yet::NotYetPlace;

use super::{Apply, Available, Category, ProfileScope, SettingDescriptor, SettingKind};

impl SettingDescriptor {
    /// A descriptor with the usual defaults (`Live`, `Always`, since 0.1, no keywords).
    #[must_use]
    pub const fn new(
        key: &'static str,
        kind: SettingKind,
        default: &'static str,
        category: Category,
        order: u16,
        label_key: &'static str,
        description_key: &'static str,
    ) -> Self {
        Self {
            key,
            kind,
            default,
            category,
            order,
            label_key,
            description_key,
            keywords: &[],
            apply: Apply::Live,
            available: Available::Always,
            since: "0.1",
            migrate_from: &[],
            hidden: false,
            profile: ProfileScope::Global,
            presets: &[],
            preset_suffix: "",
        }
    }

    /// Offers `values` as chips (the value followed by `suffix`) next to the stepper.
    #[must_use]
    pub const fn presets(mut self, values: &'static [&'static str], suffix: &'static str) -> Self {
        self.presets = values;
        self.preset_suffix = suffix;
        self
    }

    /// Sets the search keywords.
    #[must_use]
    pub const fn keywords(mut self, keywords: &'static [&'static str]) -> Self {
        self.keywords = keywords;
        self
    }

    /// Sets the apply policy.
    #[must_use]
    pub const fn apply(mut self, apply: Apply) -> Self {
        self.apply = apply;
        self
    }

    /// Marks the setting as planned.
    #[must_use]
    pub const fn not_yet(mut self, place: &'static NotYetPlace) -> Self {
        self.available = Available::NotYet(place);
        self
    }

    /// Requires a capability.
    #[must_use]
    pub const fn capability(mut self, name: &'static str) -> Self {
        self.available = Available::Capability(name);
        self
    }

    /// Names the keys this one replaces.
    #[must_use]
    pub const fn migrate_from(mut self, keys: &'static [&'static str]) -> Self {
        self.migrate_from = keys;
        self
    }

    /// Lets a map profile set the key.
    #[must_use]
    pub const fn profile(mut self, scope: ProfileScope) -> Self {
        self.profile = scope;
        self
    }

    /// Keeps the setting off the page.
    #[must_use]
    pub const fn hidden(mut self) -> Self {
        self.hidden = true;
        self
    }

    /// Sets the version that introduced the key.
    #[must_use]
    pub const fn since(mut self, since: &'static str) -> Self {
        self.since = since;
        self
    }

    /// Whether the setting holds a stored value (everything but actions).
    #[must_use]
    pub const fn is_stored(&self) -> bool {
        !matches!(self.kind, SettingKind::Action)
    }
}
