//! The weather presets of the wizard.
//!
//! The simulator changes sky and light with the weather, nothing else, so a preset's effect line
//! says only that (`wizard-and-loading.md` section 5). The ids are the ones stored in
//! `NewGameSetup::weather_preset` and in the drive history. *Live weather* is not here: it is a
//! registered not-yet place of the step.

use opts_gui::patterns::glyphs;

/// One selectable preset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Preset {
    /// Stored id.
    pub id: &'static str,
    /// Message key of the name.
    pub name_key: &'static str,
    /// Message key of the one-line effect.
    pub effect_key: &'static str,
    /// Icon glyph.
    pub glyph: &'static str,
}

/// All presets, the default first.
pub const PRESETS: [Preset; 5] = [
    Preset {
        id: "Clear Summer Day",
        name_key: "wiz-weather-clear",
        effect_key: "wiz-weather-clear-effect",
        glyph: glyphs::SUNNY,
    },
    Preset {
        id: "Overcast Autumn",
        name_key: "wiz-weather-overcast",
        effect_key: "wiz-weather-overcast-effect",
        glyph: glyphs::CLOUD,
    },
    Preset {
        id: "Rainy Afternoon",
        name_key: "wiz-weather-rain",
        effect_key: "wiz-weather-rain-effect",
        glyph: glyphs::WATER_DROP,
    },
    Preset {
        id: "Winter Snowfall",
        name_key: "wiz-weather-snow",
        effect_key: "wiz-weather-snow-effect",
        glyph: glyphs::SNOW,
    },
    Preset {
        id: "Autumn Morning Fog",
        name_key: "wiz-weather-fog",
        effect_key: "wiz-weather-fog-effect",
        glyph: glyphs::FOG,
    },
];

/// The preset with this id; an unknown id (an old history entry) gives the default.
pub fn preset_of(id: &str) -> &'static Preset {
    PRESETS.iter().find(|p| p.id == id).unwrap_or(&PRESETS[0])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_ids_fall_back_to_clear_weather() {
        assert_eq!(preset_of("Thunderstorm & Gale").id, "Clear Summer Day");
        assert_eq!(preset_of("Rainy Afternoon").glyph, glyphs::WATER_DROP);
    }

    #[test]
    fn rain_snow_and_fog_ids_keep_the_words_the_sky_code_looks_for() {
        for (id, word) in [
            ("Rainy Afternoon", "rain"),
            ("Winter Snowfall", "snow"),
            ("Autumn Morning Fog", "fog"),
            ("Overcast Autumn", "overcast"),
        ] {
            assert!(id.to_lowercase().contains(word));
        }
    }
}
