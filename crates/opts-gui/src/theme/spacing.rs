//! Material Design 3 4dp-based spacing grid tokens.

use bevy::ui::Val;

/// Semantic spacing steps snapped to a 4dp base grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpacingScale {
    None,
    XSmall,
    Small,
    Medium,
    Standard,
    Large,
    XLarge,
    Huge,
}

impl SpacingScale {
    /// Returns the pixel value for this spacing step.
    pub const fn px(&self) -> f32 {
        match self {
            Self::None => 0.0,
            Self::XSmall => 4.0,
            Self::Small => 8.0,
            Self::Medium => 12.0,
            Self::Standard => 16.0,
            Self::Large => 24.0,
            Self::XLarge => 32.0,
            Self::Huge => 48.0,
        }
    }

    /// Converts this spacing step directly into a Bevy UI [`Val::Px`].
    pub const fn val(&self) -> Val {
        Val::Px(self.px())
    }
}

/// Spacing tokens resource configuration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpacingTokens {
    pub base_unit: f32,
}

impl Default for SpacingTokens {
    fn default() -> Self {
        Self { base_unit: 4.0 }
    }
}
