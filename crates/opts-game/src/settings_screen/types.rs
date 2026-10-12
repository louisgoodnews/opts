//! Component markers and data types for the settings screen.

use bevy::prelude::*;

/// Marker component for the root container of the settings screen.
#[derive(Component)]
pub struct SettingsScreenRoot;

/// Marker component for the "Back to Main Menu" button.
#[derive(Component)]
pub struct BackToMenuButton;

/// Marker component for Theme preset buttons.
#[derive(Component)]
pub struct ThemePresetButton {
    pub preset_name: &'static str,
}

/// Marker component for per-widget color swatch buttons.
#[derive(Component)]
pub struct ColorSwatchButton {
    pub widget_type: &'static str,
    pub role_name: &'static str,
    pub hex: &'static str,
    pub color: Color,
}

/// Marker component for toggling Dark / Light theme mode in settings.
#[derive(Component)]
pub struct DarkModeToggleButton;

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
