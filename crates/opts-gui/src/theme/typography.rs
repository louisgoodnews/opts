//! Material Design 3 typography system and type scale.
//!
//! Provides the 15 baseline typography styles across Display, Headline,
//! Title, Body, and Label roles, along with the [`TypographyTokens`] struct.

use bevy::prelude::*;

/// 15 baseline typography roles defined by Material Design 3.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TypeScale {
    DisplayLarge,
    DisplayMedium,
    DisplaySmall,
    HeadlineLarge,
    HeadlineMedium,
    HeadlineSmall,
    TitleLarge,
    TitleMedium,
    TitleSmall,
    BodyLarge,
    BodyMedium,
    BodySmall,
    LabelLarge,
    LabelMedium,
    LabelSmall,
}

impl TypeScale {
    /// Returns a static slice of all 15 type scale roles.
    pub const fn all() -> &'static [TypeScale] {
        &[
            TypeScale::DisplayLarge,
            TypeScale::DisplayMedium,
            TypeScale::DisplaySmall,
            TypeScale::HeadlineLarge,
            TypeScale::HeadlineMedium,
            TypeScale::HeadlineSmall,
            TypeScale::TitleLarge,
            TypeScale::TitleMedium,
            TypeScale::TitleSmall,
            TypeScale::BodyLarge,
            TypeScale::BodyMedium,
            TypeScale::BodySmall,
            TypeScale::LabelLarge,
            TypeScale::LabelMedium,
            TypeScale::LabelSmall,
        ]
    }
}

/// Specifications for a typography style.
#[derive(Debug, Clone, PartialEq)]
pub struct TypeStyle {
    /// Nominal font size in physical/logical pixels.
    pub font_size: f32,
    /// Line height in pixels.
    pub line_height: f32,
    /// Tracking / letter spacing in pixels.
    pub tracking: f32,
    /// Optional font asset handle override.
    pub font: Option<Handle<Font>>,
}

impl TypeStyle {
    /// Creates a new type style specification.
    pub const fn new(font_size: f32, line_height: f32, tracking: f32) -> Self {
        Self {
            font_size,
            line_height,
            tracking,
            font: None,
        }
    }
}

/// Token collection providing specifications for all 15 type scale variants.
#[derive(Debug, Clone, PartialEq)]
pub struct TypographyTokens {
    styles: [TypeStyle; 15],
}

impl TypographyTokens {
    /// Maps a [`TypeScale`] role to its array index.
    const fn scale_to_index(scale: TypeScale) -> usize {
        match scale {
            TypeScale::DisplayLarge => 0,
            TypeScale::DisplayMedium => 1,
            TypeScale::DisplaySmall => 2,
            TypeScale::HeadlineLarge => 3,
            TypeScale::HeadlineMedium => 4,
            TypeScale::HeadlineSmall => 5,
            TypeScale::TitleLarge => 6,
            TypeScale::TitleMedium => 7,
            TypeScale::TitleSmall => 8,
            TypeScale::BodyLarge => 9,
            TypeScale::BodyMedium => 10,
            TypeScale::BodySmall => 11,
            TypeScale::LabelLarge => 12,
            TypeScale::LabelMedium => 13,
            TypeScale::LabelSmall => 14,
        }
    }

    /// Retrieves the style corresponding to the given scale role.
    pub fn get(&self, scale: TypeScale) -> &TypeStyle {
        &self.styles[Self::scale_to_index(scale)]
    }

    /// Mutably retrieves the style corresponding to the given scale role.
    pub fn get_mut(&mut self, scale: TypeScale) -> &mut TypeStyle {
        &mut self.styles[Self::scale_to_index(scale)]
    }
}

impl Default for TypographyTokens {
    fn default() -> Self {
        Self {
            styles: [
                TypeStyle::new(57.0, 64.0, -0.25), // DisplayLarge
                TypeStyle::new(45.0, 52.0, 0.0),   // DisplayMedium
                TypeStyle::new(36.0, 44.0, 0.0),   // DisplaySmall
                TypeStyle::new(32.0, 40.0, 0.0),   // HeadlineLarge
                TypeStyle::new(28.0, 36.0, 0.0),   // HeadlineMedium
                TypeStyle::new(24.0, 32.0, 0.0),   // HeadlineSmall
                TypeStyle::new(22.0, 28.0, 0.0),   // TitleLarge
                TypeStyle::new(16.0, 24.0, 0.15),  // TitleMedium
                TypeStyle::new(14.0, 20.0, 0.10),  // TitleSmall
                TypeStyle::new(16.0, 24.0, 0.50),  // BodyLarge
                TypeStyle::new(14.0, 20.0, 0.25),  // BodyMedium
                TypeStyle::new(12.0, 16.0, 0.40),  // BodySmall
                TypeStyle::new(14.0, 20.0, 0.10),  // LabelLarge
                TypeStyle::new(12.0, 16.0, 0.50),  // LabelMedium
                TypeStyle::new(11.0, 16.0, 0.50),  // LabelSmall
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_typography_scale_has_all_15_styles() {
        let typography = TypographyTokens::default();
        let all_scales = TypeScale::all();
        assert_eq!(all_scales.len(), 15);

        for &scale in all_scales {
            let style = typography.get(scale);
            assert!(
                style.font_size > 0.0,
                "Style {scale:?} font_size must be positive"
            );
            assert!(
                style.line_height >= style.font_size,
                "Style {scale:?} line_height must be >= font_size"
            );
        }
    }
}
