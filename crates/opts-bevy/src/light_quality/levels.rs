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

/// Bloom strength on the main camera (`graphics.bloom`). Off is the default; the other three only
/// scatter pixels brighter than display white.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BloomLevel {
    /// No bloom component on the camera (the picture is the same as without the effect).
    #[default]
    Off,
    /// A faint glow.
    Low,
    /// The restrained natural preset (the strength the old `true` setting meant).
    Natural,
    /// A stronger glow, for a deliberately bright look.
    High,
}

impl BloomLevel {
    /// All values, cheapest first.
    pub const ALL: [Self; 4] = [Self::Off, Self::Low, Self::Natural, Self::High];

    /// The stored/protocol name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Low => "low",
            Self::Natural => "natural",
            Self::High => "high",
        }
    }

    /// Parses a [`name`](Self::name). The old boolean texts are read too: `true` is
    /// [`Natural`](Self::Natural) and `false` is [`Off`](Self::Off), so stored settings and map
    /// profiles written before the enum keep their meaning.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        match text.trim() {
            "true" => Some(Self::Natural),
            "false" => Some(Self::Off),
            other => Self::ALL.into_iter().find(|b| b.name() == other),
        }
    }

    /// Whether the camera gets a bloom component at all.
    #[must_use]
    pub const fn is_on(self) -> bool {
        !matches!(self, Self::Off)
    }
}
