//! Material Design 3 colour roles and the [`ColorScheme`] that maps every role to a colour.
//!
//! Roles: the 34 M3 roles plus three semantic **status** families (`Success`, `Warning`, `Info`,
//! each with an `On…` colour and a container pair), 46 in total. The concrete palettes are built
//! in [`palette`](super::palette); contrast arithmetic lives in [`contrast`](super::contrast).

use bevy::color::Color;

use super::mode::ThemeKind;

pub use super::contrast::{contrast_ratio, relative_luminance};

/// Semantic color roles defined by Material Design 3.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ColorRole {
    // Primary group
    Primary,
    OnPrimary,
    PrimaryContainer,
    OnPrimaryContainer,

    // Secondary group
    Secondary,
    OnSecondary,
    SecondaryContainer,
    OnSecondaryContainer,

    // Tertiary group
    Tertiary,
    OnTertiary,
    TertiaryContainer,
    OnTertiaryContainer,

    // Error group
    Error,
    OnError,
    ErrorContainer,
    OnErrorContainer,

    // Surface & background group
    Surface,
    OnSurface,
    SurfaceVariant,
    OnSurfaceVariant,
    InverseSurface,
    InverseOnSurface,
    InversePrimary,
    SurfaceDim,
    SurfaceBright,
    SurfaceContainerLowest,
    SurfaceContainerLow,
    SurfaceContainer,
    SurfaceContainerHigh,
    SurfaceContainerHighest,

    // Outline & utility group
    Outline,
    OutlineVariant,
    Scrim,
    Shadow,

    // Status group (semantic roles M3 lacks)
    Success,
    OnSuccess,
    SuccessContainer,
    OnSuccessContainer,
    Warning,
    OnWarning,
    WarningContainer,
    OnWarningContainer,
    Info,
    OnInfo,
    InfoContainer,
    OnInfoContainer,
}

impl ColorRole {
    /// Returns a static slice of all standard color roles.
    pub const fn all() -> &'static [ColorRole] {
        &[
            ColorRole::Primary,
            ColorRole::OnPrimary,
            ColorRole::PrimaryContainer,
            ColorRole::OnPrimaryContainer,
            ColorRole::Secondary,
            ColorRole::OnSecondary,
            ColorRole::SecondaryContainer,
            ColorRole::OnSecondaryContainer,
            ColorRole::Tertiary,
            ColorRole::OnTertiary,
            ColorRole::TertiaryContainer,
            ColorRole::OnTertiaryContainer,
            ColorRole::Error,
            ColorRole::OnError,
            ColorRole::ErrorContainer,
            ColorRole::OnErrorContainer,
            ColorRole::Surface,
            ColorRole::OnSurface,
            ColorRole::SurfaceVariant,
            ColorRole::OnSurfaceVariant,
            ColorRole::InverseSurface,
            ColorRole::InverseOnSurface,
            ColorRole::InversePrimary,
            ColorRole::SurfaceDim,
            ColorRole::SurfaceBright,
            ColorRole::SurfaceContainerLowest,
            ColorRole::SurfaceContainerLow,
            ColorRole::SurfaceContainer,
            ColorRole::SurfaceContainerHigh,
            ColorRole::SurfaceContainerHighest,
            ColorRole::Outline,
            ColorRole::OutlineVariant,
            ColorRole::Scrim,
            ColorRole::Shadow,
            ColorRole::Success,
            ColorRole::OnSuccess,
            ColorRole::SuccessContainer,
            ColorRole::OnSuccessContainer,
            ColorRole::Warning,
            ColorRole::OnWarning,
            ColorRole::WarningContainer,
            ColorRole::OnWarningContainer,
            ColorRole::Info,
            ColorRole::OnInfo,
            ColorRole::InfoContainer,
            ColorRole::OnInfoContainer,
        ]
    }

    /// Number of roles (the length of [`ColorRole::all`]).
    pub const COUNT: usize = 46;

    /// The `On…` role whose colour is drawn on top of this role, if this role is a fill.
    pub const fn on_role(self) -> Option<ColorRole> {
        Some(match self {
            Self::Primary => Self::OnPrimary,
            Self::PrimaryContainer => Self::OnPrimaryContainer,
            Self::Secondary => Self::OnSecondary,
            Self::SecondaryContainer => Self::OnSecondaryContainer,
            Self::Tertiary => Self::OnTertiary,
            Self::TertiaryContainer => Self::OnTertiaryContainer,
            Self::Error => Self::OnError,
            Self::ErrorContainer => Self::OnErrorContainer,
            Self::Success => Self::OnSuccess,
            Self::SuccessContainer => Self::OnSuccessContainer,
            Self::Warning => Self::OnWarning,
            Self::WarningContainer => Self::OnWarningContainer,
            Self::Info => Self::OnInfo,
            Self::InfoContainer => Self::OnInfoContainer,
            Self::Surface => Self::OnSurface,
            Self::SurfaceVariant => Self::OnSurfaceVariant,
            Self::InverseSurface => Self::InverseOnSurface,
            _ => return None,
        })
    }
}

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
mod tests {
    use super::*;

    #[test]
    fn there_are_46_roles_in_declaration_order() {
        assert_eq!(ColorRole::all().len(), ColorRole::COUNT);
        for (i, role) in ColorRole::all().iter().enumerate() {
            assert_eq!(*role as usize, i, "{role:?}");
        }
    }

    #[test]
    fn every_role_of_every_theme_is_opaque() {
        for scheme in [
            ColorScheme::dark(),
            ColorScheme::light(),
            ColorScheme::high_contrast(),
        ] {
            for &role in ColorRole::all() {
                assert_eq!(scheme.get(role).to_srgba().alpha, 1.0, "{role:?}");
            }
        }
    }

    #[test]
    fn dark_is_the_default() {
        assert!(ColorScheme::default().is_dark());
        assert_eq!(ColorScheme::default().kind, ThemeKind::Dark);
    }
}
