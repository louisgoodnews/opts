//! Token marker components binding entities to Material Design 3 theme tokens.
//!
//! Entities tagged with these components have their styling (colors, borders,
//! radii, and typography) updated reactively when [`MaterialTheme`] changes.

use bevy::prelude::*;

use crate::theme::{
    color::ColorRole, elevation::ElevationLevel, shape::RadiusLevel, theme::MaterialTheme,
    typography::TypeScale,
};

/// Binds an entity's [`BackgroundColor`] to a semantic [`ColorRole`].
///
/// # Example
/// ```rust
/// use opts_gui::prelude::*;
/// use bevy::prelude::*;
///
/// fn spawn_card(mut commands: Commands) {
///     commands.spawn((
///         NodeBundle::default(),
///         BackgroundRole(ColorRole::SurfaceContainerLow),
///     ));
/// }
/// ```
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackgroundRole(pub ColorRole);

/// Binds an entity's text or icon color to a semantic [`ColorRole`].
///
/// # Example
/// ```rust
/// use opts_gui::prelude::*;
/// use bevy::prelude::*;
///
/// fn spawn_heading(mut commands: Commands) {
///     commands.spawn((
///         TextBundle::from_section("OPTS", TextStyle::default()),
///         ForegroundRole(ColorRole::Primary),
///     ));
/// }
/// ```
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ForegroundRole(pub ColorRole);

/// Binds an entity's [`BorderColor`] to a semantic [`ColorRole`].
///
/// # Example
/// ```rust
/// use opts_gui::prelude::*;
/// use bevy::prelude::*;
///
/// fn spawn_outlined(mut commands: Commands) {
///     commands.spawn((
///         NodeBundle::default(),
///         BorderRole(ColorRole::Outline),
///     ));
/// }
/// ```
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct BorderRole(pub ColorRole);

/// Binds an entity's text styling to a [`TypeScale`] role.
///
/// # Example
/// ```rust
/// use opts_gui::prelude::*;
/// use bevy::prelude::*;
///
/// fn spawn_title(mut commands: Commands) {
///     commands.spawn((
///         TextBundle::from_section("Schedule", TextStyle::default()),
///         TypeStyleToken(TypeScale::TitleMedium),
///     ));
/// }
/// ```
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct TypeStyleToken(pub TypeScale);

/// Binds an entity's [`BorderRadius`] to a [`RadiusLevel`].
///
/// # Example
/// ```rust
/// use opts_gui::prelude::*;
/// use bevy::prelude::*;
///
/// fn spawn_pill(mut commands: Commands) {
///     commands.spawn((
///         NodeBundle::default(),
///         RadiusToken(RadiusLevel::Full),
///     ));
/// }
/// ```
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct RadiusToken(pub RadiusLevel);

/// Binds an entity's elevation styling to an [`ElevationLevel`].
///
/// # Example
/// ```rust
/// use opts_gui::prelude::*;
/// use bevy::prelude::*;
///
/// fn spawn_elevated_card(mut commands: Commands) {
///     commands.spawn((
///         NodeBundle::default(),
///         ElevationToken(ElevationLevel::Level2),
///         BackgroundRole(ColorRole::SurfaceContainerLow),
///     ));
/// }
/// ```
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ElevationToken(pub ElevationLevel);

/// Query item alias for background synchronization.
pub type BackgroundSyncItem<'a> = (
    Ref<'a, BackgroundRole>,
    &'a mut BackgroundColor,
    Option<&'a ElevationToken>,
);

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
        let base_color = theme.scheme.get(role.0);
        let final_color = if let Some(elev) = elevation {
            if theme.scheme.is_dark() {
                let primary = theme.scheme.get(ColorRole::Primary);
                elev.0.apply_tonal_elevation(base_color, primary)
            } else {
                base_color
            }
        } else {
            base_color
        };
        commands.entity(entity).insert(BackgroundColor(final_color));
    }

    let theme_changed = theme.is_changed();
    for (role, mut bg, elevation) in &mut query {
        if theme_changed || role.is_changed() {
            let base_color = theme.scheme.get(role.0);
            bg.0 = if let Some(elev) = elevation {
                if theme.scheme.is_dark() {
                    let primary = theme.scheme.get(ColorRole::Primary);
                    elev.0.apply_tonal_elevation(base_color, primary)
                } else {
                    base_color
                }
            } else {
                base_color
            };
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
            .insert(BorderColor(theme.scheme.get(role.0)));
    }
    let theme_changed = theme.is_changed();
    for (role, mut border) in &mut query {
        if theme_changed || role.is_changed() {
            border.0 = theme.scheme.get(role.0);
        }
    }
}

/// System that synchronizes [`Text`] foreground color with [`ForegroundRole`].
pub fn sync_foreground_roles(
    theme: Res<MaterialTheme>,
    mut query: Query<(Ref<ForegroundRole>, &mut Text)>,
) {
    let theme_changed = theme.is_changed();
    for (role, mut text) in &mut query {
        if theme_changed || role.is_changed() {
            let color = theme.scheme.get(role.0);
            for section in &mut text.sections {
                section.style.color = color;
            }
        }
    }
}

/// System that synchronizes [`BorderRadius`] with [`RadiusToken`].
pub fn sync_radius_tokens(
    mut query: Query<(Ref<RadiusToken>, &mut BorderRadius)>,
    uninitialized: Query<(Entity, &RadiusToken), Without<BorderRadius>>,
    mut commands: Commands,
) {
    for (entity, token) in &uninitialized {
        commands.entity(entity).insert(token.0.to_border_radius());
    }
    for (token, mut radius) in &mut query {
        if token.is_changed() {
            *radius = token.0.to_border_radius();
        }
    }
}

/// System that synchronizes [`Text`] typography font size and line height with [`TypeStyleToken`].
pub fn sync_typestyle_tokens(
    theme: Res<MaterialTheme>,
    mut query: Query<(Ref<TypeStyleToken>, &mut Text)>,
) {
    let theme_changed = theme.is_changed();
    for (token, mut text) in &mut query {
        if theme_changed || token.is_changed() {
            let style = theme.typography.get(token.0);
            for section in &mut text.sections {
                section.style.font_size = style.font_size;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::ColorScheme;

    #[test]
    fn test_theme_change_updates_entities() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<MaterialTheme>()
            .add_systems(
                Update,
                (
                    sync_background_roles,
                    sync_border_roles,
                    sync_foreground_roles,
                    sync_radius_tokens,
                    sync_typestyle_tokens,
                ),
            );

        // Spawn test entity container with token components and text child
        let entity = app
            .world_mut()
            .spawn((
                NodeBundle::default(),
                BackgroundRole(ColorRole::Primary),
                BorderRole(ColorRole::Outline),
                RadiusToken(RadiusLevel::Medium),
            ))
            .id();

        let text_child = app
            .world_mut()
            .spawn((
                TextBundle::from_section("Sample Text", TextStyle::default()),
                ForegroundRole(ColorRole::OnPrimary),
                TypeStyleToken(TypeScale::BodyLarge),
            ))
            .id();

        app.world_mut().entity_mut(entity).add_child(text_child);

        // First tick sets initial values
        app.update();

        let initial_bg = app.world().get::<BackgroundColor>(entity).copied();
        assert!(
            initial_bg.is_some(),
            "BackgroundColor should have been inserted"
        );

        // Now mutate theme from Dark to Light
        {
            let mut theme = app.world_mut().resource_mut::<MaterialTheme>();
            theme.scheme = ColorScheme::light();
        }

        // Second tick updates components
        app.update();

        let updated_bg = app.world().get::<BackgroundColor>(entity).copied();
        assert!(updated_bg.is_some());
        assert_ne!(
            initial_bg.map(|b| b.0),
            updated_bg.map(|b| b.0),
            "BackgroundColor must update when MaterialTheme changes"
        );
    }
}
