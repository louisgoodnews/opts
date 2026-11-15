//! Material Design 3 colour roles and the [`ColorScheme`] that maps every role to a colour.
//!
//! Roles: the 34 M3 roles plus three semantic **status** families (`Success`, `Warning`, `Info`,
//! each with an `On…` colour and a container pair), 46 in total. The concrete palettes are built
//! in [`palette`](super::palette); contrast arithmetic lives in [`contrast`](super::contrast).

use bevy::color::Color;

use super::mode::ThemeKind;

pub use super::contrast::{contrast_ratio, relative_luminance};

mod role;

pub use role::*;

/// A complete palette mapping every semantic [`ColorRole`] to a concrete Bevy [`Color`].
#[derive(Debug, Clone, PartialEq)]
pub struct ColorScheme {
    /// `true` for dark surfaces (dark and high-contrast); equals `kind.is_dark()`.
    pub is_dark: bool,
    /// Which palette family this scheme belongs to.
    pub kind: ThemeKind,
    colors: [Color; ColorRole::COUNT],
}

impl ColorScheme {
    /// An all-black scheme of the given kind, to be filled role by role.
    pub fn blank(kind: ThemeKind) -> Self {
        Self {
            is_dark: kind.is_dark(),
            kind,
            colors: [Color::BLACK; ColorRole::COUNT],
        }
    }

    /// Gets the concrete colour for a given semantic role.
    pub fn get(&self, role: ColorRole) -> Color {
        self.colors[role as usize]
    }

    /// Sets the colour for a given semantic role.
    pub fn set(&mut self, role: ColorRole, color: Color) {
        self.colors[role as usize] = color;
    }

    /// Returns `true` if this is a dark theme (dark or high-contrast).
    pub fn is_dark(&self) -> bool {
        self.is_dark
    }

    /// The standard dark scheme with the default accent.
    pub fn dark() -> Self {
        Self::for_kind(
            ThemeKind::Dark,
            super::accent::AccentPreset::default().color(),
        )
    }

    /// The standard light scheme with the default accent.
    pub fn light() -> Self {
        Self::for_kind(
            ThemeKind::Light,
            super::accent::AccentPreset::default().color(),
        )
    }

    /// The high-contrast scheme with the default accent hue.
    pub fn high_contrast() -> Self {
        Self::for_kind(
            ThemeKind::HighContrast,
            super::accent::AccentPreset::default().color(),
        )
    }
}

impl Default for ColorScheme {
    fn default() -> Self {
        Self::dark()
    }
}

#[cfg(test)]
mod tests;
