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
