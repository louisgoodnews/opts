//! RON **theme files**: a user-editable way to change the accent and individual role colours.
//!
//! A file lives in the `themes/` folder of the configuration directory and is chosen by name in
//! the `theme_file` setting. It is versioned (`version: 1`) and passes through the contrast
//! checker of [`audit`](super::audit): a role that breaks a contrast rule is **auto-corrected**
//! (the foreground colour is moved in lightness until the rule holds) and the corrections are
//! reported; a file whose problems cannot be corrected, that does not parse, that has an unknown
//! version, an unknown role name or a malformed colour is **rejected** with a message.
//!
//! ```ron
//! (
//!     version: 1,
//!     name: "Harbour",
//!     accent: Some("#2A7F62"),
//!     dark: { "Surface": "#0E1A22", "SurfaceContainer": "#17262F" },
//!     light: {},
//!     high_contrast: {},
//! )
//! ```
//!
//! Role names are the variant names of [`ColorRole`] (`Surface`, `OnSurface`, `Primary`, ...).

use std::collections::BTreeMap;

use serde::Deserialize;

use super::color::ColorRole;
use super::mode::ThemeKind;

/// The file format version this build reads.
pub const THEME_FILE_VERSION: u32 = 1;

/// The on-disk shape of a theme file.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThemeFile {
    /// Format version; must equal [`THEME_FILE_VERSION`].
    pub version: u32,
    /// Display name.
    pub name: String,
    /// Accent seed as `#RRGGBB`; the preset accent is used when absent.
    #[serde(default)]
    pub accent: Option<String>,
    /// Role colour overrides of the dark theme (`"Role": "#RRGGBB"`).
    #[serde(default)]
    pub dark: BTreeMap<String, String>,
    /// Role colour overrides of the light theme.
    #[serde(default)]
    pub light: BTreeMap<String, String>,
    /// Role colour overrides of the high-contrast theme.
    #[serde(default)]
    pub high_contrast: BTreeMap<String, String>,
}

/// Why a theme file was rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThemeFileError {
    /// The text is not valid RON of the expected shape.
    Parse(String),
    /// The file declares a version this build does not read.
    UnsupportedVersion(u32),
    /// A colour is not `#RRGGBB`.
    BadColor {
        /// The role (or `accent`) the value belongs to.
        key: String,
        /// The offending text.
        value: String,
    },
    /// A role name is not a [`ColorRole`].
    UnknownRole(String),
    /// A contrast rule still fails after auto-correction (the colours cannot be made to work).
    Unfixable {
        /// The theme kind.
        kind: ThemeKind,
        /// The foreground role of the rule.
        fg: ColorRole,
        /// The background role of the rule.
        bg: ColorRole,
    },
}

impl std::fmt::Display for ThemeFileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Parse(e) => write!(f, "not a valid theme file: {e}"), // l10n-ok: stored identifier or diagnostic, not UI copy
            Self::UnsupportedVersion(v) => write!(f, "unsupported theme file version {v}"), // l10n-ok: diagnostic
            Self::BadColor { key, value } => {
                write!(f, "{key}: \"{value}\" is not a #RRGGBB colour") // l10n-ok: stored identifier or diagnostic, not UI copy
            }
            Self::UnknownRole(r) => write!(f, "unknown colour role \"{r}\""), // l10n-ok: stored identifier or diagnostic, not UI copy
            Self::Unfixable { kind, fg, bg } => {
                write!(
                    f,
                    "{kind:?}: {fg:?} on {bg:?} cannot reach the required contrast" // l10n-ok: stored identifier or diagnostic, not UI copy
                )
            }
        }
    }
}

impl std::error::Error for ThemeFileError {}
/// The role named `name` (the variant name, case-sensitive).
pub fn role_from_name(name: &str) -> Option<ColorRole> {
    ColorRole::all()
        .iter()
        .copied()
        .find(|r| format!("{r:?}") == name)
}

mod build;

pub use build::parse_theme_file;

#[cfg(test)]
mod tests;
