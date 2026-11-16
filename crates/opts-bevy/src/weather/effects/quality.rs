//! The precipitation quality setting: how many rain and snow quads may be drawn.

/// How many rain/snow quads the precipitation may use at most.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PrecipitationQuality {
    /// No precipitation is drawn.
    Off,
    /// 1500 quads.
    Low,
    /// 3000 quads.
    #[default]
    Medium,
    /// 6000 quads.
    High,
}

impl PrecipitationQuality {
    /// The stored value.
    #[must_use]
    pub fn key(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }

    /// The quality of a stored value.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        match text.trim() {
            "off" => Some(Self::Off),
            "low" => Some(Self::Low),
            "medium" => Some(Self::Medium),
            "high" => Some(Self::High),
            _ => None,
        }
    }

    /// The quad budget.
    #[must_use]
    pub fn budget(self) -> u32 {
        match self {
            Self::Off => 0,
            Self::Low => 1500,
            Self::Medium => 3000,
            Self::High => 6000,
        }
    }
}
