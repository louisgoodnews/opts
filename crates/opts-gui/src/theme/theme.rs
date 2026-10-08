//! MaterialTheme resource and fluent theme builder.

use bevy::prelude::*;

use super::{
    color::ColorScheme, elevation::ElevationTokens, motion::MotionTokens, shape::ShapeTokens,
    spacing::SpacingTokens, typography::TypographyTokens,
};

/// The primary design system resource holding all active Material Design 3 tokens.
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct MaterialTheme {
    /// Active semantic color scheme.
    pub scheme: ColorScheme,
    /// Typography styles across all 15 scale levels.
    pub typography: TypographyTokens,
    /// Elevation configuration and tonal overlay rules.
    pub elevation: ElevationTokens,
    /// Corner radius settings.
    pub shape: ShapeTokens,
    /// 4dp grid spacing configuration.
    pub spacing: SpacingTokens,
    /// Motion and transition settings.
    pub motion: MotionTokens,
}

impl MaterialTheme {
    /// Creates a default dark Material theme.
    pub fn dark() -> Self {
        Self {
            scheme: ColorScheme::dark(),
            typography: TypographyTokens::default(),
            elevation: ElevationTokens,
            shape: ShapeTokens,
            spacing: SpacingTokens::default(),
            motion: MotionTokens::default(),
        }
    }

    /// Creates a default light Material theme.
    pub fn light() -> Self {
        Self {
            scheme: ColorScheme::light(),
            typography: TypographyTokens::default(),
            elevation: ElevationTokens,
            shape: ShapeTokens,
            spacing: SpacingTokens::default(),
            motion: MotionTokens::default(),
        }
    }
}

impl Default for MaterialTheme {
    fn default() -> Self {
        Self::dark()
    }
}

/// Fluent builder for constructing custom [`MaterialTheme`] instances.
#[derive(Debug, Default)]
pub struct MaterialThemeBuilder {
    scheme: Option<ColorScheme>,
    typography: Option<TypographyTokens>,
    elevation: Option<ElevationTokens>,
    shape: Option<ShapeTokens>,
    spacing: Option<SpacingTokens>,
    motion: Option<MotionTokens>,
}

impl MaterialThemeBuilder {
    /// Creates a new empty theme builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the color scheme.
    pub fn with_scheme(mut self, scheme: ColorScheme) -> Self {
        self.scheme = Some(scheme);
        self
    }

    /// Sets the typography tokens.
    pub fn with_typography(mut self, typography: TypographyTokens) -> Self {
        self.typography = Some(typography);
        self
    }

    /// Sets the spacing tokens.
    pub fn with_spacing(mut self, spacing: SpacingTokens) -> Self {
        self.spacing = Some(spacing);
        self
    }

    /// Sets the motion tokens.
    pub fn with_motion(mut self, motion: MotionTokens) -> Self {
        self.motion = Some(motion);
        self
    }

    /// Builds the final [`MaterialTheme`].
    pub fn build(self) -> MaterialTheme {
        MaterialTheme {
            scheme: self.scheme.unwrap_or_else(ColorScheme::dark),
            typography: self.typography.unwrap_or_default(),
            elevation: self.elevation.unwrap_or_default(),
            shape: self.shape.unwrap_or_default(),
            spacing: self.spacing.unwrap_or_default(),
            motion: self.motion.unwrap_or_default(),
        }
    }
}
