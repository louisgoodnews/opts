//! Design-token theme system: colour roles and schemes, typography, elevation, shape, spacing,
//! motion, density and scale, theme modes and accents.
//!
//! The vocabulary is Material Design 3's; the expression is OPTS's own (see
//! `docs/src/architecture/gui-design-system.md`). Everything is unified under the
//! [`MaterialTheme`] resource.

pub mod accent;
pub mod audit;
pub mod color;
pub mod contrast;
pub mod custom;
pub mod cvd;
pub mod elevation;
#[cfg(feature = "serde")]
pub mod file;
pub mod instrument;
pub mod mode;
pub mod motion;
pub mod palette;
pub mod plugin;
pub mod runtime;
pub mod scale;
pub mod scrim;
pub mod shape;
pub mod spacing;
#[allow(clippy::module_inception)]
// Preserves exact module tree hierarchy `theme/theme.rs` specified by task requirements
pub mod theme;
pub mod typography;

pub use accent::AccentPreset;
pub use color::{ColorRole, ColorScheme};
pub use custom::CustomTheme;
pub use elevation::{ElevationLevel, ElevationTokens};
pub use instrument::{InstrumentPalette, InstrumentTokens, InstrumentVariant};
pub use mode::{theme_for, ThemeKind, ThemeMode};
pub use motion::{DurationToken, EasingCurve, MotionTokens};
pub use plugin::ThemePlugin;
pub use runtime::{SystemColorScheme, ThemeClock, ThemePreferences, ThemeSwitchLock};
pub use scale::{MinTextSize, UiScalePercent};
pub use scrim::{luminance_stats, scrim_for_text, ImageLuminance};
pub use shape::{RadiusLevel, ShapeTokens};
pub use spacing::{Density, LayoutClass, SpacingScale, SpacingTokens};
pub use theme::{MaterialTheme, MaterialThemeBuilder};
pub use typography::{TypeScale, TypeStyle, TypographyTokens};
