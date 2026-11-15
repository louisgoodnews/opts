//! Which palette is active and how it is chosen: [`ThemeKind`] (the palettes), [`ThemeMode`]
//! (the user's choice: Dark, Light, System, Auto by clock) and the pure [`theme_for`] clock rule.
//!
//! See `docs/src/roadmap/phase-9/theme-auto-switch.md` for the investigation behind the modes.

mod clock;

pub use clock::{DEFAULT_DARK_FROM, DEFAULT_LIGHT_FROM, format_time, parse_time, theme_for};

/// A concrete palette family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum ThemeKind {
    /// The default: dark surfaces, light text.
    #[default]
    Dark,
    /// Light surfaces, dark text.
    Light,
    /// The accessibility theme: text >= 7:1, controls >= 4.5:1 (dark base). Never chosen
    /// automatically.
    HighContrast,
}

impl ThemeKind {
    /// `true` when the surfaces are dark (dark and high-contrast).
    pub const fn is_dark(self) -> bool {
        !matches!(self, Self::Light)
    }

    /// Stable key used in settings and theme files.
    pub const fn key(self) -> &'static str {
        match self {
            Self::Dark => "dark",   // l10n-ok: stored identifier or diagnostic, not UI copy
            Self::Light => "light", // l10n-ok: stored identifier or diagnostic, not UI copy
            Self::HighContrast => "high_contrast", // l10n-ok: stored identifier or diagnostic, not UI copy
        }
    }

    /// Parses [`ThemeKind::key`] (case-insensitive).
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "dark" => Some(Self::Dark), // l10n-ok: stored identifier or diagnostic, not UI copy
            "light" => Some(Self::Light), // l10n-ok: stored identifier or diagnostic, not UI copy
            "high_contrast" | "high-contrast" | "highcontrast" => Some(Self::HighContrast), // l10n-ok: stored identifier or diagnostic, not UI copy
            _ => None,
        }
    }
}

/// How the application picks between dark and light (the user's choice).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum ThemeMode {
    /// Always dark (the default, decision D-UI-02).
    #[default]
    Dark,
    /// Always light.
    Light,
    /// Follow the operating system; dark when it reports nothing.
    System,
    /// Light by day, dark by night using the device's local time ([`theme_for`]).
    #[cfg_attr(feature = "serde", serde(rename = "auto"))]
    AutoClock,
}

impl ThemeMode {
    /// All modes in display order.
    pub const ALL: [ThemeMode; 4] = [Self::Dark, Self::Light, Self::System, Self::AutoClock];

    /// Stable key used in the `theme_mode` setting.
    pub const fn key(self) -> &'static str {
        match self {
            Self::Dark => "dark",   // l10n-ok: stored identifier or diagnostic, not UI copy
            Self::Light => "light", // l10n-ok: stored identifier or diagnostic, not UI copy
            Self::System => "system", // l10n-ok: stored identifier or diagnostic, not UI copy
            Self::AutoClock => "auto", // l10n-ok: stored identifier or diagnostic, not UI copy
        }
    }

    /// Parses [`ThemeMode::key`] (case-insensitive); `None` for anything else.
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "dark" => Some(Self::Dark), // l10n-ok: stored identifier or diagnostic, not UI copy
            "light" => Some(Self::Light), // l10n-ok: stored identifier or diagnostic, not UI copy
            "system" => Some(Self::System), // l10n-ok: stored identifier or diagnostic, not UI copy
            "auto" | "auto_clock" | "autoclock" => Some(Self::AutoClock), // l10n-ok: stored identifier or diagnostic, not UI copy
            _ => None,
        }
    }

    /// The message key of the display name (`crate::l10n::keys::theme`).
    pub const fn label_key(self) -> &'static str {
        use crate::l10n::keys::theme as k;
        match self {
            Self::Dark => k::MODE_DARK,
            Self::Light => k::MODE_LIGHT,
            Self::System => k::MODE_SYSTEM,
            Self::AutoClock => k::MODE_AUTO,
        }
    }

    /// Maps the legacy boolean `is_dark_theme` setting (used until M-0061) to a mode.
    pub const fn from_legacy_is_dark(is_dark: bool) -> Self {
        if is_dark { Self::Dark } else { Self::Light }
    }

    /// `true` when the mode can change the theme without the user doing anything.
    pub const fn is_automatic(self) -> bool {
        matches!(self, Self::System | Self::AutoClock)
    }
}

#[cfg(test)]
mod tests;
