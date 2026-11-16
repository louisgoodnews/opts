//! The built-in map profiles, the presets shipped with the game.

use super::*;

/// The built-in presets (not files): *Performance*, *Balanced*, *Quality*.
#[must_use]
pub fn presets() -> Vec<MapProfile> {
    let make = |id: &str, name: &str, pairs: &[(&str, &str)]| {
        let mut p = MapProfile::named(name);
        p.created = String::new();
        p.based_on = Some(id.to_string());
        p.settings = pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect();
        p
    };
    vec![
        make(
            "performance",
            "Performance",
            &[
                ("graphics.light_quality_mode", "custom"),
                ("graphics.light_shafts", "off"),
                ("graphics.light_night_shafts", "false"),
                ("graphics.light_cone_cap", "24"),
                ("graphics.light_cone_depth_fade", "false"),
                ("graphics.mirror_count", "1"),
                ("graphics.mirror_rate", "4"),
                ("graphics.glass_reflections", "false"),
                ("graphics.bloom", "false"),
                ("graphics.weather_precipitation", "low"),
                ("graphics.weather_wet_ground", "false"),
                ("texture_filtering", "4"),
                ("gameplay.traffic", "40"),
                ("gameplay.pedestrians", "10"),
                ("gameplay.tile_radius", "2"),
            ],
        ),
        make(
            "balanced",
            "Balanced",
            &[
                ("graphics.light_quality_mode", "automatic"),
                ("graphics.mirror_count", "2"),
                ("graphics.mirror_rate", "2"),
                ("graphics.weather_precipitation", "medium"),
                ("texture_filtering", "8"),
                ("gameplay.traffic", "80"),
                ("gameplay.pedestrians", "30"),
                ("gameplay.tile_radius", "0"),
            ],
        ),
        make(
            "quality",
            "Quality",
            &[
                ("graphics.light_quality_mode", "custom"),
                ("graphics.light_shafts", "high"),
                ("graphics.light_night_shafts", "true"),
                ("graphics.light_cone_cap", "96"),
                ("graphics.light_cone_depth_fade", "true"),
                ("graphics.mirror_count", "2"),
                ("graphics.mirror_rate", "1"),
                ("graphics.glass_reflections", "true"),
                ("graphics.weather_precipitation", "high"),
                ("graphics.weather_wet_ground", "true"),
                ("texture_filtering", "16"),
                ("gameplay.traffic", "120"),
                ("gameplay.pedestrians", "60"),
            ],
        ),
    ]
}
