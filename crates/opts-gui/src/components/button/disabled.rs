//! The disabled look of a text button (ISS-056). Its label is drawn in `OnSurfaceVariant`, a text
//! role that meets the theme's text contrast, and no disabled scrim covers it: the scrim (surface
//! colour at `1 - DISABLED_CONTENT`) would pull a `Primary` label below 4.5:1 on the light theme.
//! Other buttons keep the scrim.

use crate::theme::color::ColorRole;

use super::ButtonVariant;

/// The label role of a text button: `Primary` when enabled, a neutral text role when disabled.
pub fn text_label_role(disabled: bool) -> ColorRole {
    if disabled {
        ColorRole::OnSurfaceVariant
    } else {
        ColorRole::Primary
    }
}

/// Whether the disabled scrim is drawn over a control of `variant` (`None`: not a button).
pub fn scrim_drawn(variant: Option<ButtonVariant>, disabled: bool) -> bool {
    disabled && variant != Some(ButtonVariant::Text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interaction::state::DISABLED_CONTENT;
    use crate::theme::accent::{text_min, AccentPreset};
    use crate::theme::contrast::{composite_over, contrast_ratio};
    use crate::theme::{color::ColorScheme, mode::ThemeKind};
    use bevy::color::{Alpha, Color};

    /// The label colour as drawn: the role colour, under the scrim when one is drawn.
    fn drawn_label(scheme: &ColorScheme, disabled: bool) -> Color {
        let label = scheme.get(text_label_role(disabled));
        if scrim_drawn(Some(ButtonVariant::Text), disabled) {
            let scrim = scheme
                .get(ColorRole::Surface)
                .with_alpha(1.0 - DISABLED_CONTENT);
            composite_over(scrim, label)
        } else {
            label
        }
    }

    #[test]
    fn disabled_text_label_meets_the_text_minimum_on_every_theme() {
        for kind in [ThemeKind::Light, ThemeKind::Dark, ThemeKind::HighContrast] {
            let scheme = ColorScheme::for_kind(kind, AccentPreset::TransitBlue.color());
            let surface = scheme.get(ColorRole::Surface);
            let disabled = drawn_label(&scheme, true);
            let ratio = contrast_ratio(disabled, surface);
            let min = text_min(kind);
            assert!(
                ratio >= min,
                "{kind:?}: the disabled Reset label is {ratio:.2}:1 on the surface, needs {min}:1"
            );
            assert_ne!(
                disabled,
                drawn_label(&scheme, false),
                "{kind:?}: still reads as disabled"
            );
        }
    }

    #[test]
    fn only_non_text_buttons_keep_the_scrim_over_a_disabled_control() {
        assert!(!scrim_drawn(Some(ButtonVariant::Text), true));
        assert!(scrim_drawn(Some(ButtonVariant::Filled), true));
        assert!(scrim_drawn(None, true));
        assert!(!scrim_drawn(None, false));
    }
}
