//! The shadow and light modes: the shafts and light mode presets with their parsers.

/// Sun-ray (volumetric fog) quality: the ray-march step count.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Shafts {
    /// No rays (the volumetric components are removed).
    Off,
    /// 8 steps.
    Low,
    /// 16 steps (the default).
    #[default]
    Medium,
    /// 32 steps.
    High,
    /// 64 steps.
    Ultra,
}

impl Shafts {
    /// All values, cheapest first.
    pub const ALL: [Self; 5] = [Self::Off, Self::Low, Self::Medium, Self::High, Self::Ultra];

    /// The ray-march steps (0 when off).
    #[must_use]
    pub const fn steps(self) -> u32 {
        match self {
            Self::Off => 0,
            Self::Low => 8,
            Self::Medium => 16,
            Self::High => 32,
            Self::Ultra => 64,
        }
    }

    /// The stored/protocol name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Ultra => "ultra",
        }
    }

    /// Parses a [`name`](Self::name).
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|s| s.name() == text.trim())
    }
}

/// Who decides the values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LightMode {
    /// The governor picks the ladder level from the frame time (the default).
    #[default]
    Automatic,
    /// The user's controls are authoritative; the governor does nothing.
    Custom,
}

impl LightMode {
    /// The stored/protocol name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Automatic => "automatic",
            Self::Custom => "custom",
        }
    }

    /// Parses a [`name`](Self::name); anything else is `None`.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        match text.trim() {
            "automatic" => Some(Self::Automatic),
            "custom" => Some(Self::Custom),
            _ => None,
        }
    }
}
