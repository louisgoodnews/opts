//! Base palettes of the three theme kinds and [`ColorScheme::for_kind`], which completes one with
//! an accent and the status roles.
//!
//! The neutral surfaces, the error group and the tertiary group are fixed tables (the Material 3
//! baseline for dark and light, black/white for high contrast); everything accent-dependent and
//! the status roles are generated with a contrast guard ([`apply_accent`], [`apply_status`]).

use bevy::color::Color;

use super::accent::{apply_accent, colors_of, pair, SURFACES};
use super::color::{ColorRole, ColorScheme};
use super::mode::ThemeKind;

const fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::srgb_u8(r, g, b)
}

/// Sets the surface, outline, error, tertiary and inverse roles of `kind`.
fn base(scheme: &mut ColorScheme) {
    use ColorRole as R;
    let t: &[(ColorRole, Color)] = match scheme.kind {
        ThemeKind::Dark => &[
            (R::Surface, rgb(0x12, 0x13, 0x16)),
            (R::OnSurface, rgb(0xE2, 0xE2, 0xE6)),
            (R::SurfaceVariant, rgb(0x42, 0x47, 0x4F)),
            (R::OnSurfaceVariant, rgb(0xC4, 0xC6, 0xD0)),
            (R::InverseSurface, rgb(0xE2, 0xE2, 0xE6)),
            (R::InverseOnSurface, rgb(0x1A, 0x1C, 0x1E)),
            (R::SurfaceDim, rgb(0x12, 0x13, 0x16)),
            (R::SurfaceBright, rgb(0x38, 0x39, 0x3D)),
            (R::SurfaceContainerLowest, rgb(0x0D, 0x0E, 0x11)),
            (R::SurfaceContainerLow, rgb(0x1C, 0x1D, 0x21)),
            (R::SurfaceContainer, rgb(0x21, 0x1F, 0x26)),
            (R::SurfaceContainerHigh, rgb(0x2B, 0x2A, 0x2F)),
            (R::SurfaceContainerHighest, rgb(0x35, 0x34, 0x3A)),
            (R::Outline, rgb(0x93, 0x95, 0x9E)),
            (R::OutlineVariant, rgb(0x44, 0x47, 0x4E)),
            (R::Tertiary, rgb(0xD6, 0xBE, 0xE4)),
            (R::OnTertiary, rgb(0x3B, 0x29, 0x48)),
            (R::TertiaryContainer, rgb(0x52, 0x3F, 0x5F)),
            (R::OnTertiaryContainer, rgb(0xF2, 0xDA, 0xFF)),
            (R::Error, rgb(0xFF, 0xB4, 0xAB)),
            (R::OnError, rgb(0x69, 0x00, 0x05)),
            (R::ErrorContainer, rgb(0x93, 0x00, 0x0A)),
            (R::OnErrorContainer, rgb(0xFF, 0xDA, 0xD6)),
        ],
        ThemeKind::Light => &[
            (R::Surface, rgb(0xFD, 0xF8, 0xFD)),
            (R::OnSurface, rgb(0x1A, 0x1C, 0x1E)),
            (R::SurfaceVariant, rgb(0xE1, 0xE2, 0xEC)),
            (R::OnSurfaceVariant, rgb(0x43, 0x47, 0x4E)),
            (R::InverseSurface, rgb(0x2E, 0x31, 0x33)),
            (R::InverseOnSurface, rgb(0xF1, 0xF0, 0xF4)),
            (R::SurfaceDim, rgb(0xDE, 0xD8, 0xE1)),
            (R::SurfaceBright, rgb(0xFF, 0xF8, 0xFF)),
            (R::SurfaceContainerLowest, rgb(0xFF, 0xFF, 0xFF)),
            (R::SurfaceContainerLow, rgb(0xF7, 0xF2, 0xFA)),
            (R::SurfaceContainer, rgb(0xF3, 0xED, 0xF7)),
            (R::SurfaceContainerHigh, rgb(0xEC, 0xE6, 0xF0)),
            (R::SurfaceContainerHighest, rgb(0xE6, 0xE0, 0xE9)),
            (R::Outline, rgb(0x6A, 0x6E, 0x76)),
            (R::OutlineVariant, rgb(0xC4, 0xC6, 0xD0)),
            (R::Tertiary, rgb(0x6B, 0x57, 0x78)),
            (R::OnTertiary, rgb(0xFF, 0xFF, 0xFF)),
            (R::TertiaryContainer, rgb(0xF2, 0xDA, 0xFF)),
            (R::OnTertiaryContainer, rgb(0x25, 0x14, 0x32)),
            (R::Error, rgb(0xBA, 0x1A, 0x1A)),
            (R::OnError, rgb(0xFF, 0xFF, 0xFF)),
            (R::ErrorContainer, rgb(0xFF, 0xDA, 0xD6)),
            (R::OnErrorContainer, rgb(0x41, 0x00, 0x02)),
        ],
        ThemeKind::HighContrast => &[
            (R::Surface, rgb(0x00, 0x00, 0x00)),
            (R::OnSurface, rgb(0xFF, 0xFF, 0xFF)),
            (R::SurfaceVariant, rgb(0x1F, 0x1F, 0x1F)),
            (R::OnSurfaceVariant, rgb(0xE6, 0xE6, 0xE6)),
            (R::InverseSurface, rgb(0xFF, 0xFF, 0xFF)),
            (R::InverseOnSurface, rgb(0x00, 0x00, 0x00)),
            (R::SurfaceDim, rgb(0x00, 0x00, 0x00)),
            (R::SurfaceBright, rgb(0x2E, 0x2E, 0x2E)),
            (R::SurfaceContainerLowest, rgb(0x00, 0x00, 0x00)),
            (R::SurfaceContainerLow, rgb(0x0A, 0x0A, 0x0A)),
            (R::SurfaceContainer, rgb(0x12, 0x12, 0x12)),
            (R::SurfaceContainerHigh, rgb(0x1A, 0x1A, 0x1A)),
            (R::SurfaceContainerHighest, rgb(0x24, 0x24, 0x24)),
            (R::Outline, rgb(0xFF, 0xFF, 0xFF)),
            (R::OutlineVariant, rgb(0x9A, 0x9A, 0x9A)),
            (R::Tertiary, rgb(0xFF, 0xC6, 0xFF)),
            (R::OnTertiary, rgb(0x00, 0x00, 0x00)),
            (R::TertiaryContainer, rgb(0x4A, 0x1F, 0x5C)),
            (R::OnTertiaryContainer, rgb(0xFF, 0xFF, 0xFF)),
            (R::Error, rgb(0xFF, 0xA3, 0x99)),
            (R::OnError, rgb(0x00, 0x00, 0x00)),
            (R::ErrorContainer, rgb(0x7A, 0x00, 0x08)),
            (R::OnErrorContainer, rgb(0xFF, 0xFF, 0xFF)),
        ],
    };
    for &(role, color) in t {
        scheme.set(role, color);
    }
    scheme.set(R::Scrim, Color::BLACK);
    scheme.set(R::Shadow, Color::BLACK);
}

/// Derives the status families (`Success`, `Warning`, `Info`) with the contrast guard.
///
/// Hues: success green (150 deg), warning amber (80 deg), info cyan (225 deg), all in OKLCH.
pub fn apply_status(scheme: &mut ColorScheme) {
    use ColorRole as R;
    let surfaces = colors_of(scheme, &SURFACES);
    let (fill_l, container_l, on_container_l, on_l) = match scheme.kind {
        ThemeKind::Dark => (0.80, 0.34, 0.90, 0.20),
        ThemeKind::Light => (0.45, 0.91, 0.25, 0.99),
        ThemeKind::HighContrast => (0.88, 0.28, 0.97, 0.10),
    };
    for (fill, on, container, on_container, hue) in [
        (
            R::Success,
            R::OnSuccess,
            R::SuccessContainer,
            R::OnSuccessContainer,
            150.0,
        ),
        (
            R::Warning,
            R::OnWarning,
            R::WarningContainer,
            R::OnWarningContainer,
            80.0,
        ),
        (
            R::Info,
            R::OnInfo,
            R::InfoContainer,
            R::OnInfoContainer,
            225.0,
        ),
    ] {
        pair(scheme, fill, on, (hue, 0.13), (fill_l, on_l), &surfaces);
        pair(
            scheme,
            container,
            on_container,
            (hue, 0.07),
            (container_l, on_container_l),
            &[],
        );
    }
}

impl ColorScheme {
    /// Builds the complete scheme of `kind` with `accent`: base palette, accent roles, status
    /// roles, all guaranteed to meet the contrast rules of [`audit`](super::audit).
    pub fn for_kind(kind: ThemeKind, accent: Color) -> Self {
        let mut scheme = Self::blank(kind);
        base(&mut scheme);
        apply_accent(&mut scheme, accent);
        apply_status(&mut scheme);
        scheme
    }
}
