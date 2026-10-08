//! Material Design 3 corner radius and shape tokens.

use bevy::ui::{BorderRadius, Val};

/// Standard corner radius levels defined by Material Design 3.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RadiusLevel {
    None,
    ExtraSmall,
    Small,
    Medium,
    Large,
    ExtraLarge,
    Full,
}

impl RadiusLevel {
    /// Returns the corner radius in physical/logical pixels.
    pub const fn px(&self) -> f32 {
        match self {
            Self::None => 0.0,
            Self::ExtraSmall => 4.0,
            Self::Small => 8.0,
            Self::Medium => 12.0,
            Self::Large => 16.0,
            Self::ExtraLarge => 28.0,
            Self::Full => 9999.0,
        }
    }

    /// Converts this radius level to a uniform Bevy [`BorderRadius`].
    pub fn to_border_radius(&self) -> BorderRadius {
        BorderRadius::all(Val::Px(self.px()))
    }
}

/// Shape tokens configuration.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ShapeTokens;
