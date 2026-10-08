//! Material Design 3 elevation levels and tonal overlay tokens.

use bevy::color::{Color, Mix};

/// 6 standard elevation levels defined by Material Design 3 (Levels 0–5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ElevationLevel {
    #[default]
    Level0,
    Level1,
    Level2,
    Level3,
    Level4,
    Level5,
}

impl ElevationLevel {
    /// Elevation value in nominal dps.
    pub const fn dp(&self) -> f32 {
        match self {
            Self::Level0 => 0.0,
            Self::Level1 => 1.0,
            Self::Level2 => 3.0,
            Self::Level3 => 6.0,
            Self::Level4 => 8.0,
            Self::Level5 => 12.0,
        }
    }

    /// Tonal overlay tint percentage applied to surface in M3 dark theme.
    pub const fn tonal_tint_percentage(&self) -> f32 {
        match self {
            Self::Level0 => 0.0,
            Self::Level1 => 0.05,
            Self::Level2 => 0.08,
            Self::Level3 => 0.11,
            Self::Level4 => 0.12,
            Self::Level5 => 0.14,
        }
    }

    /// Blends surface background color with primary color according to tonal tint.
    pub fn apply_tonal_elevation(&self, surface: Color, primary: Color) -> Color {
        let tint = self.tonal_tint_percentage();
        if tint <= 0.0 {
            surface
        } else {
            surface.mix(&primary, tint)
        }
    }
}

/// Elevation tokens configuration.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ElevationTokens;
