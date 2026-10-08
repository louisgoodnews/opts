//! Material Design 3 color system roles and color schemes.
//!
//! Provides the [`ColorRole`] enumeration covering all semantic roles in M3,
//! and the [`ColorScheme`] struct defining light and dark baseline palettes.

use bevy::color::Color;

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
        ]
    }
}

/// A complete palette mapping every semantic [`ColorRole`] to a concrete Bevy [`Color`].
#[derive(Debug, Clone, PartialEq)]
pub struct ColorScheme {
    pub is_dark: bool,
    colors: [Color; 34],
}

impl ColorScheme {
    /// Internal index helper for [`ColorRole`].
    const fn role_to_index(role: ColorRole) -> usize {
        match role {
            ColorRole::Primary => 0,
            ColorRole::OnPrimary => 1,
            ColorRole::PrimaryContainer => 2,
            ColorRole::OnPrimaryContainer => 3,
            ColorRole::Secondary => 4,
            ColorRole::OnSecondary => 5,
            ColorRole::SecondaryContainer => 6,
            ColorRole::OnSecondaryContainer => 7,
            ColorRole::Tertiary => 8,
            ColorRole::OnTertiary => 9,
            ColorRole::TertiaryContainer => 10,
            ColorRole::OnTertiaryContainer => 11,
            ColorRole::Error => 12,
            ColorRole::OnError => 13,
            ColorRole::ErrorContainer => 14,
            ColorRole::OnErrorContainer => 15,
            ColorRole::Surface => 16,
            ColorRole::OnSurface => 17,
            ColorRole::SurfaceVariant => 18,
            ColorRole::OnSurfaceVariant => 19,
            ColorRole::InverseSurface => 20,
            ColorRole::InverseOnSurface => 21,
            ColorRole::InversePrimary => 22,
            ColorRole::SurfaceDim => 23,
            ColorRole::SurfaceBright => 24,
            ColorRole::SurfaceContainerLowest => 25,
            ColorRole::SurfaceContainerLow => 26,
            ColorRole::SurfaceContainer => 27,
            ColorRole::SurfaceContainerHigh => 28,
            ColorRole::SurfaceContainerHighest => 29,
            ColorRole::Outline => 30,
            ColorRole::OutlineVariant => 31,
            ColorRole::Scrim => 32,
            ColorRole::Shadow => 33,
        }
    }

    /// Gets the concrete color for a given semantic role.
    pub fn get(&self, role: ColorRole) -> Color {
        self.colors[Self::role_to_index(role)]
    }

    /// Sets the color for a given semantic role.
    pub fn set(&mut self, role: ColorRole, color: Color) {
        self.colors[Self::role_to_index(role)] = color;
    }

    /// Returns `true` if this is a dark theme.
    pub fn is_dark(&self) -> bool {
        self.is_dark
    }

    /// Creates the standard Material Design 3 Light color scheme.
    pub fn light() -> Self {
        let mut scheme = Self {
            is_dark: false,
            colors: [Color::BLACK; 34],
        };

        scheme.set(ColorRole::Primary, Color::srgb(0.0, 0.38, 0.64)); // #0061A4
        scheme.set(ColorRole::OnPrimary, Color::WHITE);
        scheme.set(ColorRole::PrimaryContainer, Color::srgb(0.82, 0.89, 1.0)); // #D1E4FF
        scheme.set(ColorRole::OnPrimaryContainer, Color::srgb(0.0, 0.11, 0.21)); // #001D36

        scheme.set(ColorRole::Secondary, Color::srgb(0.33, 0.37, 0.44)); // #535F70
        scheme.set(ColorRole::OnSecondary, Color::WHITE);
        scheme.set(ColorRole::SecondaryContainer, Color::srgb(0.84, 0.89, 0.97)); // #D7E3F7
        scheme.set(
            ColorRole::OnSecondaryContainer,
            Color::srgb(0.06, 0.11, 0.17),
        ); // #101C2B

        scheme.set(ColorRole::Tertiary, Color::srgb(0.42, 0.34, 0.47)); // #6B5778
        scheme.set(ColorRole::OnTertiary, Color::WHITE);
        scheme.set(ColorRole::TertiaryContainer, Color::srgb(0.95, 0.85, 1.0)); // #F2DAFF
        scheme.set(
            ColorRole::OnTertiaryContainer,
            Color::srgb(0.15, 0.08, 0.20),
        ); // #251432

        scheme.set(ColorRole::Error, Color::srgb(0.73, 0.10, 0.10)); // #BA1A1A
        scheme.set(ColorRole::OnError, Color::WHITE);
        scheme.set(ColorRole::ErrorContainer, Color::srgb(1.0, 0.85, 0.84)); // #FFDAD6
        scheme.set(ColorRole::OnErrorContainer, Color::srgb(0.25, 0.0, 0.01)); // #410002

        scheme.set(ColorRole::Surface, Color::srgb(0.99, 0.97, 0.99)); // #FDF8FD
        scheme.set(ColorRole::OnSurface, Color::srgb(0.10, 0.11, 0.12)); // #1A1C1E
        scheme.set(ColorRole::SurfaceVariant, Color::srgb(0.88, 0.89, 0.91)); // #E1E2EC
        scheme.set(ColorRole::OnSurfaceVariant, Color::srgb(0.26, 0.28, 0.31)); // #43474E
        scheme.set(ColorRole::InverseSurface, Color::srgb(0.18, 0.19, 0.20));
        scheme.set(ColorRole::InverseOnSurface, Color::srgb(0.95, 0.95, 0.96));
        scheme.set(ColorRole::InversePrimary, Color::srgb(0.62, 0.79, 1.0));

        scheme.set(ColorRole::SurfaceDim, Color::srgb(0.87, 0.85, 0.88));
        scheme.set(ColorRole::SurfaceBright, Color::srgb(1.0, 0.97, 1.0));
        scheme.set(ColorRole::SurfaceContainerLowest, Color::WHITE);
        scheme.set(
            ColorRole::SurfaceContainerLow,
            Color::srgb(0.97, 0.95, 0.98),
        );
        scheme.set(ColorRole::SurfaceContainer, Color::srgb(0.95, 0.93, 0.97));
        scheme.set(
            ColorRole::SurfaceContainerHigh,
            Color::srgb(0.93, 0.90, 0.94),
        );
        scheme.set(
            ColorRole::SurfaceContainerHighest,
            Color::srgb(0.90, 0.88, 0.91),
        );

        scheme.set(ColorRole::Outline, Color::srgb(0.45, 0.47, 0.50)); // #73777F
        scheme.set(ColorRole::OutlineVariant, Color::srgb(0.77, 0.78, 0.82)); // #C4C6D0
        scheme.set(ColorRole::Scrim, Color::BLACK);
        scheme.set(ColorRole::Shadow, Color::BLACK);

        scheme
    }

    /// Creates the standard Material Design 3 Dark color scheme.
    pub fn dark() -> Self {
        let mut scheme = Self {
            is_dark: true,
            colors: [Color::BLACK; 34],
        };

        scheme.set(ColorRole::Primary, Color::srgb(0.62, 0.79, 1.0)); // #9FCAFF
        scheme.set(ColorRole::OnPrimary, Color::srgb(0.0, 0.20, 0.35)); // #003258
        scheme.set(ColorRole::PrimaryContainer, Color::srgb(0.0, 0.29, 0.49)); // #00497D
        scheme.set(ColorRole::OnPrimaryContainer, Color::srgb(0.82, 0.89, 1.0)); // #D1E4FF

        scheme.set(ColorRole::Secondary, Color::srgb(0.73, 0.78, 0.86)); // #BBC7DB
        scheme.set(ColorRole::OnSecondary, Color::srgb(0.15, 0.19, 0.25)); // #253140
        scheme.set(ColorRole::SecondaryContainer, Color::srgb(0.23, 0.28, 0.35)); // #3B4858
        scheme.set(
            ColorRole::OnSecondaryContainer,
            Color::srgb(0.84, 0.89, 0.97),
        ); // #D7E3F7

        scheme.set(ColorRole::Tertiary, Color::srgb(0.84, 0.75, 0.89)); // #D6BEE4
        scheme.set(ColorRole::OnTertiary, Color::srgb(0.23, 0.16, 0.28)); // #3B2948
        scheme.set(ColorRole::TertiaryContainer, Color::srgb(0.32, 0.25, 0.37)); // #523F5F
        scheme.set(ColorRole::OnTertiaryContainer, Color::srgb(0.95, 0.85, 1.0)); // #F2DAFF

        scheme.set(ColorRole::Error, Color::srgb(1.0, 0.71, 0.67)); // #FFB4AB
        scheme.set(ColorRole::OnError, Color::srgb(0.41, 0.0, 0.02)); // #690005
        scheme.set(ColorRole::ErrorContainer, Color::srgb(0.58, 0.0, 0.04)); // #93000A
        scheme.set(ColorRole::OnErrorContainer, Color::srgb(1.0, 0.85, 0.84)); // #FFDAD6

        scheme.set(ColorRole::Surface, Color::srgb(0.07, 0.07, 0.09)); // #121316
        scheme.set(ColorRole::OnSurface, Color::srgb(0.89, 0.89, 0.90)); // #E2E2E6
        scheme.set(ColorRole::SurfaceVariant, Color::srgb(0.26, 0.28, 0.31));
        scheme.set(ColorRole::OnSurfaceVariant, Color::srgb(0.77, 0.78, 0.82)); // #C4C6D0
        scheme.set(ColorRole::InverseSurface, Color::srgb(0.89, 0.89, 0.90));
        scheme.set(ColorRole::InverseOnSurface, Color::srgb(0.10, 0.11, 0.12));
        scheme.set(ColorRole::InversePrimary, Color::srgb(0.0, 0.38, 0.64));

        scheme.set(ColorRole::SurfaceDim, Color::srgb(0.07, 0.07, 0.09));
        scheme.set(ColorRole::SurfaceBright, Color::srgb(0.22, 0.22, 0.24));
        scheme.set(
            ColorRole::SurfaceContainerLowest,
            Color::srgb(0.05, 0.05, 0.07),
        );
        scheme.set(
            ColorRole::SurfaceContainerLow,
            Color::srgb(0.11, 0.11, 0.13),
        );
        scheme.set(ColorRole::SurfaceContainer, Color::srgb(0.13, 0.12, 0.15));
        scheme.set(
            ColorRole::SurfaceContainerHigh,
            Color::srgb(0.17, 0.16, 0.19),
        );
        scheme.set(
            ColorRole::SurfaceContainerHighest,
            Color::srgb(0.21, 0.20, 0.23),
        );

        scheme.set(ColorRole::Outline, Color::srgb(0.56, 0.56, 0.60)); // #8E9099
        scheme.set(ColorRole::OutlineVariant, Color::srgb(0.27, 0.28, 0.31)); // #44474E
        scheme.set(ColorRole::Scrim, Color::BLACK);
        scheme.set(ColorRole::Shadow, Color::BLACK);

        scheme
    }
}

impl Default for ColorScheme {
    fn default() -> Self {
        Self::dark()
    }
}

/// Converts an sRGB color component in [0.0, 1.0] to linear light.
fn srgb_to_linear(channel: f32) -> f32 {
    if channel <= 0.04045 {
        channel / 12.92
    } else {
        ((channel + 0.055) / 1.055).powf(2.4)
    }
}

/// Calculates the WCAG 2.1 relative luminance of a color.
pub fn relative_luminance(color: Color) -> f32 {
    let srgba = color.to_srgba();
    let r = srgb_to_linear(srgba.red);
    let g = srgb_to_linear(srgba.green);
    let b = srgb_to_linear(srgba.blue);
    0.2126 * r + 0.7152 * g + 0.0722 * b
}

/// Calculates the contrast ratio between two colors according to WCAG 2.1 standards.
///
/// Returns a value between 1.0 (no contrast) and 21.0 (black on white).
pub fn contrast_ratio(c1: Color, c2: Color) -> f32 {
    let l1 = relative_luminance(c1);
    let l2 = relative_luminance(c2);
    let (lighter, darker) = if l1 > l2 { (l1, l2) } else { (l2, l1) };
    (lighter + 0.05) / (darker + 0.05)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_color_roles_populated() {
        let light = ColorScheme::light();
        let dark = ColorScheme::dark();

        for &role in ColorRole::all() {
            let c_light = light.get(role);
            let c_dark = dark.get(role);

            // Verify alpha is 1.0 (fully opaque standard roles)
            assert_eq!(
                c_light.to_srgba().alpha,
                1.0,
                "Light role {role:?} alpha must be 1.0"
            );
            assert_eq!(
                c_dark.to_srgba().alpha,
                1.0,
                "Dark role {role:?} alpha must be 1.0"
            );
        }
    }

    #[test]
    fn test_wcag_aa_contrast_ratios() {
        let light = ColorScheme::light();
        let dark = ColorScheme::dark();

        // WCAG AA requires at least 4.5:1 for normal text and icons
        let pairs = [
            (
                ColorRole::Primary,
                ColorRole::OnPrimary,
                "Primary/OnPrimary",
            ),
            (
                ColorRole::Secondary,
                ColorRole::OnSecondary,
                "Secondary/OnSecondary",
            ),
            (
                ColorRole::Tertiary,
                ColorRole::OnTertiary,
                "Tertiary/OnTertiary",
            ),
            (ColorRole::Error, ColorRole::OnError, "Error/OnError"),
            (
                ColorRole::Surface,
                ColorRole::OnSurface,
                "Surface/OnSurface",
            ),
        ];

        for (bg_role, fg_role, name) in pairs {
            let light_ratio = contrast_ratio(light.get(bg_role), light.get(fg_role));
            assert!(
                light_ratio >= 4.5,
                "Light mode {name} contrast ratio {light_ratio:.2} is below WCAG AA minimum 4.5:1"
            );

            let dark_ratio = contrast_ratio(dark.get(bg_role), dark.get(fg_role));
            assert!(
                dark_ratio >= 4.5,
                "Dark mode {name} contrast ratio {dark_ratio:.2} is below WCAG AA minimum 4.5:1"
            );
        }
    }
}
