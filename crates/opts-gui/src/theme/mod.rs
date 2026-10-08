//! Material Design 3 theme system, tokens, and resources.
//!
//! Exposes token definitions for color, typography, elevation, shape,
//! spacing, and motion, unified under the [`MaterialTheme`] resource.

pub mod color;
pub mod elevation;
pub mod motion;
pub mod shape;
pub mod spacing;
#[allow(clippy::module_inception)]
// Preserves exact module tree hierarchy `theme/theme.rs` specified by task requirements
pub mod theme;
pub mod typography;

pub use color::{ColorRole, ColorScheme};
pub use elevation::{ElevationLevel, ElevationTokens};
pub use motion::{DurationToken, EasingCurve, MotionTokens};
pub use shape::{RadiusLevel, ShapeTokens};
pub use spacing::{SpacingScale, SpacingTokens};
pub use theme::{MaterialTheme, MaterialThemeBuilder};
pub use typography::{TypeScale, TypeStyle, TypographyTokens};
