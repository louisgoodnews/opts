//! A custom theme: one colour scheme per theme kind, as produced by a theme file
//! (`file::parse_theme_file`) and stored in the [`ThemePreferences`](super::ThemePreferences).

use bevy::color::Color;

use super::color::{ColorRole, ColorScheme};
use super::mode::ThemeKind;

/// A loaded, contrast-checked custom theme: one scheme per theme kind.
#[derive(Debug, Clone, PartialEq)]
pub struct CustomTheme {
    /// Display name from the file.
    pub name: String,
    /// The accent in force (the file's, if any).
    pub accent: Option<Color>,
    /// Dark, light and high-contrast schemes, in that order.
    pub schemes: [ColorScheme; 3],
    /// Roles the checker had to change, per theme kind.
    pub corrected: Vec<(ThemeKind, ColorRole)>,
}

impl CustomTheme {
    /// The scheme of `kind`.
    pub fn scheme(&self, kind: ThemeKind) -> &ColorScheme {
        match kind {
            ThemeKind::Dark => &self.schemes[0],
            ThemeKind::Light => &self.schemes[1],
            ThemeKind::HighContrast => &self.schemes[2],
        }
    }
}
