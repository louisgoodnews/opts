//! Glassmorphic surface components and color synchronization systems.

use bevy::prelude::*;
use opts_gui::theme::{color::ColorRole, theme::MaterialTheme};

/// Marker component defining a translucent surface whose background alpha
/// allows the underlying map background to show through.
#[derive(Component, Debug, Clone, Copy)]
pub struct GlassSurface {
    /// Semantic color role providing the base hue.
    pub role: ColorRole,
    /// Translucent surface alpha value (0.0 .. 1.0).
    pub alpha: f32,
}

impl GlassSurface {
    /// Creates a new glassmorphic surface configuration.
    pub const fn new(role: ColorRole, alpha: f32) -> Self {
        Self { role, alpha }
    }
}

/// System that synchronizes [`BackgroundColor`] with [`GlassSurface`] and [`MaterialTheme`].
///
/// Converts the theme token's solid color to an `srgba` color with the configured alpha.
pub fn sync_glass_surfaces(
    theme: Res<MaterialTheme>,
    mut query: Query<(&GlassSurface, &mut BackgroundColor)>,
    uninitialized: Query<(Entity, &GlassSurface), Without<BackgroundColor>>,
    mut commands: Commands,
) {
    // Initialize newly spawned glass surfaces
    for (entity, glass) in &uninitialized {
        let base_color = theme.scheme.get(glass.role);
        let glass_color = base_color.with_alpha(glass.alpha);
        commands.entity(entity).insert(BackgroundColor(glass_color));
    }

    // React to theme changes
    if theme.is_changed() {
        for (glass, mut bg) in &mut query {
            let base_color = theme.scheme.get(glass.role);
            bg.0 = base_color.with_alpha(glass.alpha);
        }
    }
}
