//! Component markers and data types for the settings screen.

use bevy::prelude::*;

/// Marker component for the root container of the settings screen.
#[derive(Component)]
pub struct SettingsScreenRoot;

/// Marker component for the "Back to Main Menu" button.
#[derive(Component)]
pub struct BackToMenuButton;

/// One option of the theme and interface settings (see `theme_choice.rs`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeChoice {
    /// Dark, Light, System or Auto (clock).
    Mode(opts_gui::theme::ThemeMode),
    /// The high-contrast theme off or on.
    HighContrast(bool),
    /// An accent preset.
    Accent(opts_gui::theme::AccentPreset),
    /// UI scale in percent.
    Scale(u32),
    /// Interface density.
    Density(opts_gui::theme::Density),
    /// Reduced motion off or on.
    ReducedMotion(bool),
    /// Moves the "light from" time of the auto mode by this many minutes.
    LightFromStep(i32),
    /// Moves the "dark from" time of the auto mode by this many minutes.
    DarkFromStep(i32),
}

/// Marker component of a button choosing a [`ThemeChoice`].
#[derive(Component)]
pub struct ThemeChoiceButton(pub ThemeChoice);

/// Marker component of a button choosing a theme file (`None`: the built-in theme).
#[derive(Component)]
pub struct ThemeFileButton(pub Option<String>);

/// Marker of the text showing an auto-mode switch time (`true`: light from, `false`: dark from).
#[derive(Component)]
pub struct AutoTimeLabel(pub bool);

/// Marker component for setting the vehicle discovery interval in days.
#[derive(Component)]
pub struct DiscoveryIntervalButton {
    pub interval_days: u32,
}

/// Marker component for the "Run Discovery Now" button.
#[derive(Component)]
pub struct RunDiscoveryNowButton;

/// One option of a rendering setting (see `render_choice.rs`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderChoice {
    /// Texture filtering quality.
    Filtering(opts_bevy::TextureFiltering),
    /// Colour look.
    Look(crate::look::Look),
}

impl RenderChoice {
    /// Whether two choices belong to the same setting (radio group).
    #[must_use]
    pub fn same_group(self, other: Self) -> bool {
        std::mem::discriminant(&self) == std::mem::discriminant(&other)
    }

    /// The label of the button.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Filtering(f) => f.label(),
            Self::Look(l) => l.label(),
        }
    }

    /// Whether this option is the one stored in `settings`.
    #[must_use]
    pub fn is_selected(self, settings: &crate::settings::GameSettings) -> bool {
        match self {
            Self::Filtering(f) => settings.texture_filtering == f,
            Self::Look(l) => settings.look == l,
        }
    }
}

/// Marker component of a button choosing a [`RenderChoice`].
#[derive(Component)]
pub struct RenderChoiceButton(pub RenderChoice);
