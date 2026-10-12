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

use bevy::color::Color;
use serde::Deserialize;

use super::audit::{audit, auto_correct};
use super::color::{ColorRole, ColorScheme};
use super::contrast::{parse_hex, to_hex};
use super::custom::CustomTheme;
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

fn build_scheme(
    kind: ThemeKind,
    accent: Color,
    overrides: &BTreeMap<String, String>,
) -> Result<(ColorScheme, Vec<ColorRole>), ThemeFileError> {
    let mut scheme = ColorScheme::for_kind(kind, accent);
    let before = scheme.clone();
    for (name, hex) in overrides {
        let role = role_from_name(name).ok_or_else(|| ThemeFileError::UnknownRole(name.clone()))?;
        let color = parse_hex(hex).ok_or_else(|| ThemeFileError::BadColor {
            key: name.clone(),
            value: hex.clone(),
        })?;
        scheme.set(role, color);
    }
    let fixed_roles = auto_correct(&mut scheme);
    if let Some(v) = audit(&scheme).first() {
        return Err(ThemeFileError::Unfixable {
            kind,
            fg: v.rule.fg,
            bg: v.rule.bg,
        });
    }
    // Report only roles that differ from what the user asked for (override or generated).
    let mut asked = before;
    for (name, hex) in overrides {
        if let (Some(r), Some(c)) = (role_from_name(name), parse_hex(hex)) {
            asked.set(r, c);
        }
    }
    let corrected = fixed_roles
        .into_iter()
        .filter(|&r| to_hex(asked.get(r)) != to_hex(scheme.get(r)))
        .collect();
    Ok((scheme, corrected))
}

/// Parses and checks a theme file.
pub fn parse_theme_file(text: &str) -> Result<CustomTheme, ThemeFileError> {
    let file: ThemeFile = ron::from_str(text).map_err(|e| ThemeFileError::Parse(e.to_string()))?;
    if file.version != THEME_FILE_VERSION {
        return Err(ThemeFileError::UnsupportedVersion(file.version));
    }
    let accent = match &file.accent {
        Some(hex) => Some(parse_hex(hex).ok_or_else(|| ThemeFileError::BadColor {
            key: "accent".into(), // l10n-ok: stored identifier or diagnostic, not UI copy
            value: hex.clone(),
        })?),
        None => None,
    };
    let seed = accent.unwrap_or_else(|| super::accent::AccentPreset::default().color());
    let mut corrected = Vec::new();
    let mut schemes = Vec::new();
    for (kind, overrides) in [
        (ThemeKind::Dark, &file.dark),
        (ThemeKind::Light, &file.light),
        (ThemeKind::HighContrast, &file.high_contrast),
    ] {
        let (scheme, fixed) = build_scheme(kind, seed, overrides)?;
        corrected.extend(fixed.into_iter().map(|r| (kind, r)));
        schemes.push(scheme);
    }
    let [dark, light, hc]: [ColorScheme; 3] = schemes
        .try_into()
        .map_err(|_| ThemeFileError::Parse("internal: three schemes expected".into()))?; // l10n-ok: stored identifier or diagnostic, not UI copy
    Ok(CustomTheme {
        name: file.name,
        accent,
        schemes: [dark, light, hc],
        corrected,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::contrast::contrast_ratio;

    #[test]
    fn a_minimal_file_loads_and_is_sound() {
        let t = parse_theme_file(r#"(version: 1, name: "Plain")"#).expect("loads");
        assert_eq!(t.name, "Plain");
        for kind in [ThemeKind::Dark, ThemeKind::Light, ThemeKind::HighContrast] {
            assert!(audit(t.scheme(kind)).is_empty());
        }
        assert!(t.corrected.is_empty());
    }

    #[test]
    fn an_accent_and_overrides_apply() {
        let t = parse_theme_file(
            r##"(version: 1, name: "H", accent: Some("#2A7F62"), dark: {"Surface": "#0E1A22"})"##,
        )
        .expect("loads");
        assert_eq!(
            to_hex(t.scheme(ThemeKind::Dark).get(ColorRole::Surface)),
            "#0E1A22"
        );
        assert!(t.accent.is_some());
    }

    #[test]
    fn a_bad_pair_is_corrected_and_reported() {
        // yellow text on the light-blue dark-theme primary: about 1.3:1
        let t = parse_theme_file(r##"(version: 1, name: "Loud", dark: {"OnPrimary": "#FFFF00"})"##)
            .expect("auto-corrected");
        let s = t.scheme(ThemeKind::Dark);
        assert!(contrast_ratio(s.get(ColorRole::OnPrimary), s.get(ColorRole::Primary)) >= 4.5);
        assert!(t
            .corrected
            .contains(&(ThemeKind::Dark, ColorRole::OnPrimary)));
        assert!(audit(s).is_empty());
        // only the dark theme was touched
        assert!(t.corrected.iter().all(|(k, _)| *k == ThemeKind::Dark));
    }

    #[test]
    fn colours_that_cannot_work_are_rejected() {
        // a near-white surface in the dark theme: no single text colour reads on it and on the
        // dark containers at the same time
        let e = parse_theme_file(r##"(version: 1, name: "Bright", dark: {"Surface": "#F0F0F0"})"##)
            .expect_err("unfixable");
        assert!(
            matches!(
                e,
                ThemeFileError::Unfixable {
                    kind: ThemeKind::Dark,
                    ..
                }
            ),
            "{e}"
        );
    }

    #[test]
    fn broken_files_are_rejected_with_a_reason() {
        for (text, check) in [
            ("(version: 2, name: \"x\")", "version"),
            ("garbage", "not a valid theme file"),
            (
                "(version: 1, name: \"x\", dark: {\"Nope\": \"#000000\"})",
                "unknown colour role",
            ),
            (
                "(version: 1, name: \"x\", dark: {\"Surface\": \"red\"})",
                "#RRGGBB",
            ),
            ("(version: 1, name: \"x\", accent: Some(\"12\"))", "#RRGGBB"),
            (
                "(version: 1, name: \"x\", colour: 3)",
                "not a valid theme file",
            ),
        ] {
            let e = parse_theme_file(text).expect_err(text).to_string();
            assert!(e.contains(check), "{text}: {e}");
        }
    }

    #[test]
    fn role_names_are_the_variant_names() {
        assert_eq!(role_from_name("OnPrimary"), Some(ColorRole::OnPrimary));
        assert_eq!(role_from_name("onprimary"), None);
        for r in ColorRole::all() {
            assert_eq!(role_from_name(&format!("{r:?}")), Some(*r));
        }
    }
}
