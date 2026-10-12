//! The [`MaterialTheme`] resource: every active token set, and its builder.

use bevy::prelude::*;

use super::{
    accent::AccentPreset,
    color::ColorScheme,
    elevation::ElevationTokens,
    instrument::{InstrumentTokens, InstrumentVariant},
    mode::ThemeKind,
    motion::MotionTokens,
    shape::ShapeTokens,
    spacing::SpacingTokens,
    typography::TypographyTokens,
};

/// The primary design system resource holding all active tokens.
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct MaterialTheme {
    /// Active semantic colour scheme (46 roles).
    pub scheme: ColorScheme,
    /// The nine text styles.
    pub typography: TypographyTokens,
    /// Elevation configuration.
    pub elevation: ElevationTokens,
    /// Corner radius settings.
    pub shape: ShapeTokens,
    /// 4 px grid spacing configuration.
    pub spacing: SpacingTokens,
    /// Motion and transition settings.
    pub motion: MotionTokens,
    /// Colours and sizes of HUD instruments (separate from the application theme).
    pub instruments: InstrumentTokens,
}

impl MaterialTheme {
    /// A theme of `kind` with `accent`.
    pub fn new(kind: ThemeKind, accent: AccentPreset) -> Self {
        Self::with_scheme(ColorScheme::for_kind(kind, accent.color()))
    }

    /// A theme around an existing colour scheme (default tokens otherwise).
    pub fn with_scheme(scheme: ColorScheme) -> Self {
        Self {
            scheme,
            typography: TypographyTokens::default(),
            elevation: ElevationTokens,
            shape: ShapeTokens,
            spacing: SpacingTokens::default(),
            motion: MotionTokens::default(),
            instruments: InstrumentTokens::default(),
        }
    }

    /// The default dark theme.
    pub fn dark() -> Self {
        Self::with_scheme(ColorScheme::dark())
    }

    /// The default light theme.
    pub fn light() -> Self {
        Self::with_scheme(ColorScheme::light())
    }

    /// The high-contrast theme.
    pub fn high_contrast() -> Self {
        Self::with_scheme(ColorScheme::high_contrast())
    }

    /// Which palette family is active.
    pub fn kind(&self) -> ThemeKind {
        self.scheme.kind
    }

    /// The palette of the given instrument variant.
    pub fn instrument(&self, variant: InstrumentVariant) -> &super::instrument::InstrumentPalette {
        self.instruments.palette(variant)
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
    instruments: Option<InstrumentTokens>,
}

impl MaterialThemeBuilder {
    /// Creates a new empty theme builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the colour scheme.
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

    /// Sets the instrument tokens.
    pub fn with_instruments(mut self, instruments: InstrumentTokens) -> Self {
        self.instruments = Some(instruments);
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
            instruments: self.instruments.unwrap_or_default(),
        }
    }
}
