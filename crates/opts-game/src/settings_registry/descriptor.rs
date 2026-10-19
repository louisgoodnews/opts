//! The declarative description of one setting (see `docs/src/architecture/settings-registry.md`).

use opts_gui::not_yet::NotYetPlace;

/// The categories of the settings page, in display order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Category {
    /// Window, rendering, look.
    Graphics,
    /// Volumes (appears with audio, M-0027).
    Audio,
    /// Key, gamepad and wheel bindings.
    Controls,
    /// Theme, language, units, hints.
    Interface,
    /// Clock, traffic, assists.
    Gameplay,
    /// OMSI folder, catalogue, cache.
    Content,
    /// Developer panel, logging, file locations.
    Advanced,
}

impl Category {
    /// All categories in display order.
    pub const ALL: [Self; 7] = [
        Self::Graphics,
        Self::Audio,
        Self::Controls,
        Self::Interface,
        Self::Gameplay,
        Self::Content,
        Self::Advanced,
    ];

    /// The stable id (`graphics`), used in deep links, ids and the generated reference.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Graphics => "graphics",
            Self::Audio => "audio",
            Self::Controls => "controls",
            Self::Interface => "interface",
            Self::Gameplay => "gameplay",
            Self::Content => "content",
            Self::Advanced => "advanced",
        }
    }

    /// The message key of the category name.
    #[must_use]
    pub const fn label_key(self) -> &'static str {
        match self {
            Self::Graphics => "settings-cat-graphics",
            Self::Audio => "settings-cat-audio",
            Self::Controls => "settings-cat-controls",
            Self::Interface => "settings-cat-interface",
            Self::Gameplay => "settings-cat-gameplay",
            Self::Content => "settings-cat-content",
            Self::Advanced => "settings-cat-advanced",
        }
    }

    /// Parses an [`id`](Self::id).
    #[must_use]
    pub fn parse(id: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|c| c.id().eq_ignore_ascii_case(id.trim()))
    }
}

/// One option of an enumerated setting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnumOption {
    /// The stored text.
    pub value: &'static str,
    /// The message key of the label.
    pub label_key: &'static str,
}

/// Shorthand for an [`EnumOption`].
#[must_use]
pub const fn opt(value: &'static str, label_key: &'static str) -> EnumOption {
    EnumOption { value, label_key }
}

/// What a setting holds; decides validation and the control of the page.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SettingKind {
    /// On or off (`true` / `false`).
    Bool,
    /// One of a fixed list.
    Enum(&'static [EnumOption]),
    /// A whole number in a range.
    Int {
        /// Smallest value.
        min: i64,
        /// Largest value.
        max: i64,
        /// Step of the stepper control.
        step: i64,
    },
    /// A decimal number in a range.
    Float {
        /// Smallest value.
        min: f64,
        /// Largest value.
        max: f64,
        /// Step of the stepper control.
        step: f64,
    },
    /// Free text, at most 256 characters.
    Text,
    /// A file or folder path; empty means none.
    Path,
    /// A time of day, `HH:MM`.
    Time,
    /// An input binding (`Key:KeyW`, `Pad:South`, empty = unbound).
    Binding,
    /// A button without a stored value.
    Action,
}

/// When a changed value takes effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Apply {
    /// At once.
    Live,
    /// After a restart; the row says so.
    OnRestart,
    /// At once, but the user must confirm within this many seconds or the old value returns.
    WithRevert(u32),
}

/// Whether a setting may differ per map (`docs/src/architecture/map-profiles.md` §1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProfileScope {
    /// The person's or the machine's: never in a map profile.
    #[default]
    Global,
    /// Changes performance or the simulated content of a map: a map profile may set it.
    Map,
    /// Defines the simulation itself: never in a map profile.
    Locked,
}

/// Whether a setting can be used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Available {
    /// Always.
    Always,
    /// Needs a capability of the machine or the build (`audio`, `gamepad`); see
    /// [`super::SettingsCapabilities`].
    Capability(&'static str),
    /// Planned: shown with the red "Feature not available yet!".
    NotYet(&'static NotYetPlace),
}

/// The declaration of one setting.
#[derive(Debug, Clone, Copy)]
pub struct SettingDescriptor {
    /// Stable key (`graphics.vsync`); the keys of earlier versions keep their stored name.
    pub key: &'static str,
    /// What it holds.
    pub kind: SettingKind,
    /// The stored text of the default value.
    pub default: &'static str,
    /// Page category.
    pub category: Category,
    /// Order inside the category.
    pub order: u16,
    /// Message key of the label; `<label_key>-desc` is the description unless
    /// [`description_key`](Self::description_key) says otherwise.
    pub label_key: &'static str,
    /// Message key of the one-line description.
    pub description_key: &'static str,
    /// Extra English search words.
    pub keywords: &'static [&'static str],
    /// When a change takes effect.
    pub apply: Apply,
    /// Whether it can be used.
    pub available: Available,
    /// Version that introduced the key.
    pub since: &'static str,
    /// Older key names read when this key is absent.
    pub migrate_from: &'static [&'static str],
    /// Kept out of the page and the search (internal state such as the last map); still
    /// available to the control protocol and the generated reference.
    pub hidden: bool,
    /// Whether a map profile may set it (default [`ProfileScope::Global`]).
    pub profile: ProfileScope,
    /// Stored values of a whole-number setting that the page offers as chips next to the
    /// stepper (the stepper stays for every other value); empty for none.
    pub presets: &'static [&'static str],
    /// Text after a preset chip's number (`" %"`).
    pub preset_suffix: &'static str,
}

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
