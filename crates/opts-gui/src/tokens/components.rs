//! Token marker components binding entities to Material Design 3 theme tokens.
//!
//! Entities tagged with these components have their styling (colors, borders,
//! radii, and typography) updated reactively when [`MaterialTheme`] changes.

use bevy::prelude::*;
use bevy::text::{FontFeatureTag, FontFeatures, LetterSpacing, LineHeight};

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
///         Node::default(),
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
///         (Text::new("OPTS")),
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
///         Node::default(),
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
///         (Text::new("Schedule")),
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
///         Node::default(),
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
///         Node::default(),
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

/// Query item alias for elevation shadow synchronization.
pub type ElevationSyncItem<'a> = (Entity, Ref<'a, ElevationToken>, Option<&'a BoxShadow>);

/// Query item alias for text style synchronization.
pub type TypeStyleSyncItem<'a> = (
    Entity,
    Ref<'a, TypeStyleToken>,
    &'a mut TextFont,
    Option<&'a mut LineHeight>,
    Option<&'a mut LetterSpacing>,
    Option<&'a TypeWeight>,
);

/// System that gives every raised entity (an [`ElevationToken`] above level 0) its drop
/// shadow, and refreshes it when the level or the theme changes.
pub fn sync_elevation_shadows(
    theme: Res<MaterialTheme>,
    mut commands: Commands,
    query: Query<ElevationSyncItem, With<Node>>,
) {
    let theme_changed = theme.is_changed();
    for (entity, level, current) in &query {
        if !(theme_changed || level.is_changed()) && current.is_some() {
            continue;
        }
        match level.0.shadow(&theme.scheme) {
            Some(shadow) => {
                if current != Some(&shadow) {
                    commands.entity(entity).insert(shadow);
                }
            }
            None if current.is_some() => {
                commands.entity(entity).remove::<BoxShadow>();
            }
            None => {}
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

/// System that synchronizes the node's border radius with [`RadiusToken`].
pub fn sync_radius_tokens(mut query: Query<(Ref<RadiusToken>, &mut Node)>) {
    for (token, mut node) in &mut query {
        let radius = token.0.to_border_radius();
        if token.is_changed() && node.border_radius != radius {
            node.border_radius = radius;
        }
    }
}

/// Overrides the weight of a [`TypeStyleToken`] text (otherwise the style's own weight applies).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct TypeWeight(pub u16);

/// System that applies a [`TypeStyleToken`]'s full style to its text: size, line height, letter
/// spacing, weight, family and tabular figures.
///
/// Text whose font is neither the default one nor the UI family (the icon font, for instance)
/// keeps its font and only receives size, line height and tracking.
pub fn sync_typestyle_tokens(
    theme: Res<MaterialTheme>,
    typography: Option<Res<crate::fonts::UiTypography>>,
    mut commands: Commands,
    mut query: Query<TypeStyleSyncItem>,
) {
    // The user's text preferences (family and base size) restyle all text when they change.
    let theme_changed = theme.is_changed() || typography.as_ref().is_some_and(|t| t.is_changed());
    let typo = typography.as_deref().cloned().unwrap_or_default();
    for (entity, token, mut font, line_height, spacing, weight) in &mut query {
        if !(theme_changed || token.is_changed()) {
            continue;
        }
        let style = theme.typography.resolve(token.0);
        font.font_size = FontSize::Px(typo.size(style.font_size));
        if crate::fonts::is_ui_font(&font.font) {
            font.font = typo.font_source();
            font.weight = FontWeight(weight.map_or(style.weight, |w| w.0));
            font.font_features = if style.tabular {
                FontFeatures::builder()
                    .enable(FontFeatureTag::TABULAR_FIGURES)
                    .build()
            } else {
                FontFeatures::default()
            };
        }
        let ratio = LineHeight::RelativeToFont(style.line_height_ratio());
        match line_height {
            Some(mut lh) => {
                if *lh != ratio {
                    *lh = ratio;
                }
            }
            None => {
                commands.entity(entity).insert(ratio);
            }
        }
        let tracking = LetterSpacing::Px(style.tracking);
        match spacing {
            Some(mut ls) => {
                if *ls != tracking {
                    *ls = tracking;
                }
            }
            None => {
                commands.entity(entity).insert(tracking);
            }
        }
    }
}

#[cfg(test)]
mod tests;
