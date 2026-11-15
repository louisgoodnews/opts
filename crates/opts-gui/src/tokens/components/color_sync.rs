//! Colour token systems: background (with the tonal tint of the elevation), border and text colour.

use bevy::prelude::*;

use crate::theme::{color::ColorRole, theme::MaterialTheme};

use super::{BackgroundRole, BorderRole, ElevationToken, ForegroundRole};

/// Query item alias for background synchronization.
pub type BackgroundSyncItem<'a> = (
    Ref<'a, BackgroundRole>,
    &'a mut BackgroundColor,
    Option<&'a ElevationToken>,
);

/// The colour of a background role at an optional elevation: the role colour with the tonal tint
/// of the level mixed in (in **both** themes).
fn background_color(
    theme: &MaterialTheme,
    role: ColorRole,
    elevation: Option<&ElevationToken>,
) -> Color {
    let base = theme.scheme.get(role);
    // The high-contrast theme has no tonal tint: the tint moves the surface off the colour the
    // contrast audit guarantees (inverse accent on the inverse surface measured 6.89:1 against
    // the required 7:1 with it); shadows and outlines carry the elevation there.
    if theme.scheme.kind == crate::theme::mode::ThemeKind::HighContrast {
        return base;
    }
    match elevation {
        Some(level) => level
            .0
            .apply_tonal_elevation(base, theme.scheme.get(ColorRole::Primary)),
        None => base,
    }
}

/// System that synchronizes [`BackgroundColor`] with [`BackgroundRole`].
pub fn sync_background_roles(
    theme: Res<MaterialTheme>,
    mut query: Query<BackgroundSyncItem>,
    uninitialized: Query<
        (Entity, &BackgroundRole, Option<&ElevationToken>),
        Without<BackgroundColor>,
    >,
    mut commands: Commands,
) {
    for (entity, role, elevation) in &uninitialized {
        commands
            .entity(entity)
            .insert(BackgroundColor(background_color(&theme, role.0, elevation)));
    }

    let theme_changed = theme.is_changed();
    for (role, mut bg, elevation) in &mut query {
        if theme_changed || role.is_changed() {
            bg.0 = background_color(&theme, role.0, elevation);
        }
    }
}
/// System that synchronizes [`BorderColor`] with [`BorderRole`].
pub fn sync_border_roles(
    theme: Res<MaterialTheme>,
    mut query: Query<(Ref<BorderRole>, &mut BorderColor)>,
    uninitialized: Query<(Entity, &BorderRole), Without<BorderColor>>,
    mut commands: Commands,
) {
    for (entity, role) in &uninitialized {
        commands
            .entity(entity)
            .insert(BorderColor::all(theme.scheme.get(role.0)));
    }
    let theme_changed = theme.is_changed();
    for (role, mut border) in &mut query {
        if theme_changed || role.is_changed() {
            *border = BorderColor::all(theme.scheme.get(role.0));
        }
    }
}

/// System that synchronizes [`Text`] foreground color with [`ForegroundRole`].
pub fn sync_foreground_roles(
    theme: Res<MaterialTheme>,
    mut query: Query<(Ref<ForegroundRole>, &mut TextColor)>,
) {
    let theme_changed = theme.is_changed();
    for (role, mut text) in &mut query {
        if theme_changed || role.is_changed() {
            text.0 = theme.scheme.get(role.0);
        }
    }
}
