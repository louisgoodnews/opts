//! Component markers of the settings screen.

use bevy::prelude::*;

/// Marker component for the root container of the settings screen.
#[derive(Component)]
pub struct SettingsScreenRoot;

/// Marker component for the "Back" button (shown when no shell offers navigation).
#[derive(Component)]
pub struct BackToMenuButton;
