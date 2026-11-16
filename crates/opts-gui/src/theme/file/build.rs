//! Building a theme from a parsed theme file: the colour scheme, the correction and the checks.

use std::collections::BTreeMap;

use bevy::color::Color;

use super::super::audit::{audit, auto_correct};
use super::super::color::{ColorRole, ColorScheme};
use super::super::contrast::{parse_hex, to_hex};
use super::super::custom::CustomTheme;
use super::super::mode::ThemeKind;
use super::{role_from_name, ThemeFile, ThemeFileError, THEME_FILE_VERSION};

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
    let seed = accent.unwrap_or_else(|| super::super::accent::AccentPreset::default().color());
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
