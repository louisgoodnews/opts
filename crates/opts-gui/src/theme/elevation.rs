//! Elevation: tonal tint in **both** themes plus a real drop shadow ([`BoxShadow`]).
//!
//! A raised surface is drawn lighter-by-tint (its base colour mixed with `Primary`) and, from
//! level 1 up, casts a shadow whose strength depends on the theme (a shadow is hard to see on a
//! dark surface, so dark themes use a denser one).

use bevy::color::{Alpha, Color, Mix};
use bevy::ui::{BoxShadow, Val};

use super::color::{ColorRole, ColorScheme};

/// The six elevation levels (0 flat to 5 dialogs).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, PartialOrd, Ord)]
pub enum ElevationLevel {
    /// Flat.
    #[default]
    Level0,
    /// Cards at rest.
    Level1,
    /// Hovered cards.
    Level2,
    /// Menus, popups.
    Level3,
    /// Navigation drawers, sheets.
    Level4,
    /// Dialogs.
    Level5,
}

impl ElevationLevel {
    /// Elevation value in nominal dp.
    pub const fn dp(&self) -> f32 {
        match self {
            Self::Level0 => 0.0,
            Self::Level1 => 1.0,
            Self::Level2 => 3.0,
            Self::Level3 => 6.0,
            Self::Level4 => 8.0,
            Self::Level5 => 12.0,
        }
    }

    /// Tonal tint fraction mixed from `Primary` into the surface.
    pub const fn tonal_tint_percentage(&self) -> f32 {
        match self {
            Self::Level0 => 0.0,
            Self::Level1 => 0.05,
            Self::Level2 => 0.08,
            Self::Level3 => 0.11,
            Self::Level4 => 0.12,
            Self::Level5 => 0.14,
        }
    }

    /// `(y offset, blur)` of the shadow in logical pixels (`None` for level 0).
    pub const fn shadow_geometry(&self) -> Option<(f32, f32)> {
        match self {
            Self::Level0 => None,
            Self::Level1 => Some((1.0, 3.0)),
            Self::Level2 => Some((2.0, 6.0)),
            Self::Level3 => Some((4.0, 8.0)),
            Self::Level4 => Some((6.0, 10.0)),
            Self::Level5 => Some((8.0, 14.0)),
        }
    }

    /// Blends the surface background colour with the primary colour according to the tint.
    pub fn apply_tonal_elevation(&self, surface: Color, primary: Color) -> Color {
        let tint = self.tonal_tint_percentage();
        if tint <= 0.0 {
            surface
        } else {
            surface.mix(&primary, tint)
        }
    }

    /// The tinted background of `role` in `scheme` at this level.
    pub fn tinted(&self, scheme: &ColorScheme, role: ColorRole) -> Color {
        self.apply_tonal_elevation(scheme.get(role), scheme.get(ColorRole::Primary))
    }

    /// The shadow of this level in `scheme`, or `None` for level 0.
    pub fn shadow(&self, scheme: &ColorScheme) -> Option<BoxShadow> {
        let (y, blur) = self.shadow_geometry()?;
        let strength = if scheme.is_dark() { 0.60 } else { 0.28 };
        let color = scheme.get(ColorRole::Shadow).with_alpha(strength);
        Some(BoxShadow::new(
            color,
            Val::Px(0.0),
            Val::Px(y),
            Val::Px(0.0),
            Val::Px(blur),
        ))
    }
}

/// Elevation tokens configuration.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ElevationTokens;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::contrast::contrast_ratio;

    #[test]
    fn level_zero_is_flat_and_the_rest_cast_a_shadow() {
        let s = ColorScheme::dark();
        assert!(ElevationLevel::Level0.shadow(&s).is_none());
        for l in [
            ElevationLevel::Level1,
            ElevationLevel::Level2,
            ElevationLevel::Level3,
            ElevationLevel::Level4,
            ElevationLevel::Level5,
        ] {
            assert!(l.shadow(&s).is_some(), "{l:?}");
        }
    }

    #[test]
    fn tint_applies_in_both_themes_and_keeps_text_readable() {
        for s in [ColorScheme::dark(), ColorScheme::light()] {
            for l in [ElevationLevel::Level1, ElevationLevel::Level5] {
                let tinted = l.tinted(&s, ColorRole::SurfaceContainerHigh);
                assert_ne!(tinted, s.get(ColorRole::SurfaceContainerHigh));
                assert!(
                    contrast_ratio(tinted, s.get(ColorRole::OnSurface)) >= 4.5,
                    "{l:?} {:?}",
                    s.kind
                );
            }
        }
    }

    #[test]
    fn dark_shadows_are_denser() {
        let alpha = |s: &ColorScheme| {
            ElevationLevel::Level3
                .shadow(s)
                .and_then(|b| b.0.first().map(|st| st.color.alpha()))
                .unwrap_or(0.0)
        };
        assert!(alpha(&ColorScheme::dark()) > alpha(&ColorScheme::light()));
    }
}
