//! What a preset does to the sky: its [`WeatherKind`], for the fallback light and the icons.

/// What a preset does to the sky, for classification (fallback light, icons).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WeatherKind {
    /// Clear sky.
    Clear,
    /// Overcast.
    Overcast,
    /// Rain.
    Rain,
    /// Snow.
    Snow,
    /// Fog.
    Fog,
}

impl WeatherKind {
    /// The kind of a preset id, by the words it contains (`rain`, `snow`, `fog`, `overcast`);
    /// anything else is clear.
    #[must_use]
    pub fn of_preset(id: &str) -> Self {
        let lower = id.to_lowercase();
        if lower.contains("snow") {
            Self::Snow
        } else if lower.contains("rain") {
            Self::Rain
        } else if lower.contains("fog") {
            Self::Fog
        } else if lower.contains("overcast") {
            Self::Overcast
        } else {
            Self::Clear
        }
    }

    /// Whether the sky is covered (the fallback light turns diffuse).
    #[must_use]
    pub fn overcast(self) -> bool {
        self != Self::Clear
    }
}
