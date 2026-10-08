//! Material Design 3 named color constants and reference tokens.

use bevy::color::Color;

/// Baseline reference palette colors from Material Design 3.
pub mod baseline {
    use super::*;

    // Blue tonal palette (Primary reference)
    pub const BLUE_10: Color = Color::srgb(0.0, 0.11, 0.21);
    pub const BLUE_20: Color = Color::srgb(0.0, 0.20, 0.35);
    pub const BLUE_30: Color = Color::srgb(0.0, 0.29, 0.49);
    pub const BLUE_40: Color = Color::srgb(0.0, 0.38, 0.64);
    pub const BLUE_80: Color = Color::srgb(0.62, 0.79, 1.0);
    pub const BLUE_90: Color = Color::srgb(0.82, 0.89, 1.0);

    // Neutral tonal palette (Surface reference)
    pub const NEUTRAL_6: Color = Color::srgb(0.05, 0.05, 0.07);
    pub const NEUTRAL_10: Color = Color::srgb(0.07, 0.07, 0.09);
    pub const NEUTRAL_12: Color = Color::srgb(0.11, 0.11, 0.13);
    pub const NEUTRAL_17: Color = Color::srgb(0.13, 0.12, 0.15);
    pub const NEUTRAL_22: Color = Color::srgb(0.17, 0.16, 0.19);
    pub const NEUTRAL_90: Color = Color::srgb(0.89, 0.89, 0.90);
    pub const NEUTRAL_94: Color = Color::srgb(0.95, 0.93, 0.97);
    pub const NEUTRAL_96: Color = Color::srgb(0.97, 0.95, 0.98);
    pub const NEUTRAL_98: Color = Color::srgb(0.99, 0.97, 0.99);
    pub const NEUTRAL_99: Color = Color::srgb(1.0, 0.98, 1.0);
    pub const WHITE: Color = Color::srgb(1.0, 1.0, 1.0);
    pub const BLACK: Color = Color::srgb(0.0, 0.0, 0.0);
}
