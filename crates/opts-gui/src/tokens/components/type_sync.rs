//! Type token systems: the full text style of a [`TypeStyleToken`] and the weight override.

use bevy::prelude::*;
use bevy::text::{FontFeatureTag, FontFeatures, LetterSpacing, LineHeight};

use crate::theme::theme::MaterialTheme;

use super::TypeStyleToken;

/// Query item alias for text style synchronization.
pub type TypeStyleSyncItem<'a> = (
    Entity,
    Ref<'a, TypeStyleToken>,
    &'a mut TextFont,
    Option<&'a mut LineHeight>,
    Option<&'a mut LetterSpacing>,
    Option<&'a TypeWeight>,
);

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
