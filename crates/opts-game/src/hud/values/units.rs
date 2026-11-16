//! The speed unit of the readout (`interface.units`).

/// Speed unit of the readout (`interface.units`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SpeedUnit {
    /// Kilometres per hour.
    #[default]
    Kmh,
    /// Miles per hour.
    Mph,
}

impl SpeedUnit {
    /// From the stored setting text.
    #[must_use]
    pub fn parse(text: Option<&str>) -> Self {
        match text {
            Some("mph") => Self::Mph,
            _ => Self::Kmh,
        }
    }

    /// Message key of the unit caption.
    #[must_use]
    pub const fn caption_key(self) -> &'static str {
        match self {
            Self::Kmh => crate::hud::keys::UNIT_KMH,
            Self::Mph => crate::hud::keys::UNIT_MPH,
        }
    }

    /// `kmh` converted to this unit.
    #[must_use]
    pub fn convert(self, kmh: f32) -> f32 {
        match self {
            Self::Kmh => kmh,
            Self::Mph => kmh * 0.621_371,
        }
    }
}
