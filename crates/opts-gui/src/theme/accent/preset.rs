//! The named accent presets offered in the settings, with their key and seed colour.

use bevy::color::Color;

use crate::theme::contrast::parse_hex;

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
