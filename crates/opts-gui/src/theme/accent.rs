//! Accent presets and the generator that derives the accent-dependent roles of a scheme from one
//! seed colour while **guaranteeing** contrast.
//!
//! Only `Primary`, `OnPrimary`, `PrimaryContainer`, `OnPrimaryContainer`, `Secondary…` and
//! `InversePrimary` depend on the accent; every other role comes from the base palette.
//! [`apply_accent`] searches the lightness of each derived colour until its pairs reach the
//! thresholds of the scheme's [`ThemeKind`] (4.5:1 text, 7:1 in high contrast), so no preset can
//! produce an unreadable theme (audit finding UI-F09: Amber 2.15:1, Emerald 2.57:1).

use bevy::color::{Color, Oklcha};

use super::color::{ColorRole, ColorScheme};
use super::contrast::{ensure_contrast, min_contrast, oklch, parse_hex};
use super::mode::ThemeKind;

/// A named accent colour offered in the settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AccentPreset {
    /// "Transit blue", the default.
    #[default]
    TransitBlue,
    /// Amber.
    Amber,
    /// Violet.
    Violet,
    /// Emerald.
    Emerald,
    /// Teal.
    Teal,
}

impl AccentPreset {
    /// All presets in display order.
    pub const ALL: [AccentPreset; 5] = [
        Self::TransitBlue,
        Self::Amber,
        Self::Violet,
        Self::Emerald,
        Self::Teal,
    ];

    /// The key stored in the `active_theme` setting (the old preset names are kept).
    pub const fn key(self) -> &'static str {
        match self {
            Self::TransitBlue => "Default", // l10n-ok: stored identifier or diagnostic, not UI copy
            Self::Amber => "Amber",         // l10n-ok: stored identifier or diagnostic, not UI copy
            Self::Violet => "Violet",       // l10n-ok: stored identifier or diagnostic, not UI copy
            Self::Emerald => "Emerald",     // l10n-ok: stored identifier or diagnostic, not UI copy
            Self::Teal => "Teal",           // l10n-ok: stored identifier or diagnostic, not UI copy
        }
    }

    /// The message key of the display name (`crate::l10n::keys::theme`).
    pub const fn label_key(self) -> &'static str {
        use crate::l10n::keys::theme as k;
        match self {
            Self::TransitBlue => k::ACCENT_DEFAULT,
            Self::Amber => k::ACCENT_AMBER,
            Self::Violet => k::ACCENT_VIOLET,
            Self::Emerald => k::ACCENT_EMERALD,
            Self::Teal => k::ACCENT_TEAL,
        }
    }

    /// Parses a stored key; unknown names give `None` (callers fall back to the default).
    pub fn parse(key: &str) -> Option<Self> {
        let key = key.trim();
        Self::ALL
            .into_iter()
            .find(|p| p.key().eq_ignore_ascii_case(key))
            // The English display name was stored by one earlier version.
            .or_else(|| {
                key.eq_ignore_ascii_case("Transit blue") // l10n-ok: stored identifier of earlier versions
                    .then_some(Self::TransitBlue)
            })
    }

    /// The seed colour of the preset.
    pub fn color(self) -> Color {
        let hex = match self {
            Self::TransitBlue => "#0061A4",
            Self::Amber => "#F59E0B", // l10n-ok: stored identifier or diagnostic, not UI copy
            Self::Violet => "#8B5CF6", // l10n-ok: stored identifier or diagnostic, not UI copy
            Self::Emerald => "#10B981",
            Self::Teal => "#0E94A8",
        };
        parse_hex(hex).unwrap_or(Color::srgb(0.0, 0.38, 0.64))
    }
}

/// The surface roles accent text has to be readable on.
pub const SURFACES: [ColorRole; 9] = [
    ColorRole::Surface,
    ColorRole::SurfaceDim,
    ColorRole::SurfaceBright,
    ColorRole::SurfaceContainerLowest,
    ColorRole::SurfaceContainerLow,
    ColorRole::SurfaceContainer,
    ColorRole::SurfaceContainerHigh,
    ColorRole::SurfaceContainerHighest,
    ColorRole::SurfaceVariant,
];

/// Minimum text contrast of a theme kind.
pub const fn text_min(kind: ThemeKind) -> f32 {
    match kind {
        ThemeKind::HighContrast => 7.0,
        _ => 4.5,
    }
}

/// Minimum contrast of control boundaries and icons of a theme kind.
pub const fn control_min(kind: ThemeKind) -> f32 {
    match kind {
        ThemeKind::HighContrast => 4.5,
        _ => 3.0,
    }
}

/// The colours of `roles` in `scheme`.
pub fn colors_of(scheme: &ColorScheme, roles: &[ColorRole]) -> Vec<Color> {
    roles.iter().map(|&r| scheme.get(r)).collect()
}

/// Sets `fill` to a colour of hue `hue` and (at most) chroma `chroma` whose text colour pair has the
/// required contrast; both are searched from the starting lightnesses, `on` away from `fill`.
pub(super) fn pair(
    scheme: &mut ColorScheme,
    fill: ColorRole,
    on: ColorRole,
    (hue, chroma): (f32, f32),
    (fill_l, on_l): (f32, f32),
    fill_must_read_on: &[Color],
) {
    let min = text_min(scheme.kind);
    let dark = scheme.is_dark();
    let mut f = oklch(fill_l, chroma, hue);
    if !fill_must_read_on.is_empty() {
        f = ensure_contrast(f, fill_must_read_on, min, dark);
    }
    let o = oklch(on_l, chroma * 0.4, hue);
    // The `On` colour moves away from the fill: dark text on a light fill, light on a dark one.
    let on_lighter = relative_lum(f) < 0.18;
    let o = ensure_contrast(o, &[f], min, on_lighter);
    // If the lightness search ran into the end of its range, fall back to black or white.
    let o = if min_contrast(o, &[f]) >= min {
        o
    } else if on_lighter {
        Color::WHITE
    } else {
        Color::BLACK
    };
    scheme.set(fill, f);
    scheme.set(on, o);
}

fn relative_lum(c: Color) -> f32 {
    super::contrast::relative_luminance(c)
}

/// Derives the accent-dependent roles of `scheme` from `accent`.
///
/// Hue and (capped) chroma come from the seed; lightness is chosen per theme kind and then
/// adjusted until every pair meets [`text_min`]. `Primary` additionally has to be readable *as
/// text* on every surface, because the UI draws links and active labels in it.
pub fn apply_accent(scheme: &mut ColorScheme, accent: Color) {
    let seed: Oklcha = accent.into();
    let hue = seed.hue;
    let chroma = seed.chroma.clamp(0.04, 0.17);
    let surfaces = colors_of(scheme, &SURFACES);
    let (primary_l, container_l, on_container_l) = match scheme.kind {
        ThemeKind::Dark => (0.80, 0.38, 0.90),
        ThemeKind::Light => (0.48, 0.90, 0.25),
        ThemeKind::HighContrast => (0.90, 0.30, 0.97),
    };
    let on_primary_l = if scheme.is_dark() { 0.22 } else { 0.99 };
    pair(
        scheme,
        ColorRole::Primary,
        ColorRole::OnPrimary,
        (hue, chroma),
        (primary_l, on_primary_l),
        &surfaces,
    );
    pair(
        scheme,
        ColorRole::PrimaryContainer,
        ColorRole::OnPrimaryContainer,
        (hue, chroma * 0.8),
        (container_l, on_container_l),
        &[],
    );
    // Secondary: the same hue, much quieter.
    let (sec_l, sec_container_l) = match scheme.kind {
        ThemeKind::Dark => (0.80, 0.34),
        ThemeKind::Light => (0.46, 0.91),
        ThemeKind::HighContrast => (0.88, 0.28),
    };
    pair(
        scheme,
        ColorRole::Secondary,
        ColorRole::OnSecondary,
        (hue, 0.035),
        (sec_l, on_primary_l),
        &surfaces,
    );
    pair(
        scheme,
        ColorRole::SecondaryContainer,
        ColorRole::OnSecondaryContainer,
        (hue, 0.03),
        (sec_container_l, on_container_l),
        &[],
    );
    // The inverse accent is the other theme's primary (used on `InverseSurface`).
    let inverse_l = if scheme.is_dark() { 0.48 } else { 0.80 };
    let inverse = oklch(inverse_l, chroma, hue);
    let inverse = ensure_contrast(
        inverse,
        &[scheme.get(ColorRole::InverseSurface)],
        text_min(scheme.kind),
        !scheme.is_dark(),
    );
    scheme.set(ColorRole::InversePrimary, inverse);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::contrast::contrast_ratio;

    #[test]
    fn presets_parse_by_key() {
        for p in AccentPreset::ALL {
            assert_eq!(AccentPreset::parse(p.key()), Some(p));
        }
        assert_eq!(AccentPreset::parse("Pink"), None);
        assert_eq!(AccentPreset::default(), AccentPreset::TransitBlue);
    }

    #[test]
    fn a_very_bright_and_a_very_dark_seed_both_read() {
        for kind in [ThemeKind::Dark, ThemeKind::Light, ThemeKind::HighContrast] {
            for seed in ["#FFFF00", "#000000", "#FFFFFF", "#808080", "#00FF00"] {
                let mut scheme = ColorScheme::for_kind(kind, AccentPreset::default().color());
                apply_accent(&mut scheme, parse_hex(seed).expect("hex"));
                let r = contrast_ratio(
                    scheme.get(ColorRole::Primary),
                    scheme.get(ColorRole::OnPrimary),
                );
                assert!(r >= text_min(kind), "{kind:?} {seed}: {r:.2}");
            }
        }
    }
}
