//! Graphics settings: water and the weather effects.

use super::*;

/// Water and the weather effects.
pub(super) fn world() -> Vec<D> {
    vec![
        D::new(
            "graphics.water",
            K::Bool,
            "true",
            C::Graphics,
            70,
            "set-water",
            "set-water-desc",
        )
        .keywords(&["river", "lake", "sea", "sea level"])
        .apply(Apply::OnRestart)
        .since("0.3"),
        D::new(
            "graphics.weather_fog",
            K::Bool,
            "true",
            C::Graphics,
            72,
            "set-weather-fog",
            "set-weather-fog-desc",
        )
        .keywords(&["weather", "mist", "visibility", "haze"])
        .since("0.3"),
        D::new(
            "graphics.weather_clouds",
            K::Bool,
            "true",
            C::Graphics,
            74,
            "set-weather-clouds",
            "set-weather-clouds-desc",
        )
        .keywords(&["weather", "sky", "overcast", "cloud layer"])
        .since("0.3"),
        D::new(
            "graphics.weather_precipitation",
            K::Enum(PRECIPITATION),
            "medium",
            C::Graphics,
            76,
            "set-weather-precipitation",
            "set-weather-precipitation-desc",
        )
        .keywords(&["weather", "rain", "snow", "particles", "drops"])
        .since("0.3"),
        D::new(
            "graphics.weather_wet_ground",
            K::Bool,
            "true",
            C::Graphics,
            78,
            "set-weather-wet-ground",
            "set-weather-wet-ground-desc",
        )
        .keywords(&["weather", "wet", "puddles", "snow cover", "road"])
        .since("0.3"),
        D::new(
            "graphics.weather_wet_objects",
            K::Bool,
            "false",
            C::Graphics,
            81,
            "set-weather-wet-objects",
            "set-weather-wet-objects-desc",
        )
        .keywords(&[
            "weather",
            "wet",
            "vehicles",
            "buildings",
            "objects",
            "gloss",
        ])
        .since("0.3"),
    ]
}

/// Rain and snow intensity and the wind in the trees.
pub(super) fn weather_strength() -> Vec<D> {
    vec![
        D::new(
            "graphics.rain_intensity",
            K::Int {
                min: 0,
                max: 200,
                step: 10,
            },
            "100",
            C::Graphics,
            79,
            "set-rain-intensity",
            "set-rain-intensity-desc",
        )
        .keywords(&["weather", "rain", "drops", "wet", "puddles", "intensity"])
        .since("0.5"),
        D::new(
            "graphics.snow_intensity",
            K::Int {
                min: 0,
                max: 200,
                step: 10,
            },
            "100",
            C::Graphics,
            80,
            "set-snow-intensity",
            "set-snow-intensity-desc",
        )
        .keywords(&["weather", "snow", "flakes", "cover", "intensity"])
        .since("0.5"),
        D::new(
            "graphics.tree_wind",
            K::Bool,
            "true",
            C::Graphics,
            82,
            "set-tree-wind",
            "set-tree-wind-desc",
        )
        .keywords(&["trees", "wind", "sway", "vegetation", "animation"])
        .since("0.5"),
    ]
}
