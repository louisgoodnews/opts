//! The Graphics page settings (window, texture filtering, look, water, weather).

use super::*;

/// Part 1: graphics, audio, controls.
pub(super) fn graphics() -> Vec<D> {
    vec![
        D::new(
            "graphics.window_mode",
            K::Enum(WINDOW_MODES),
            "windowed",
            C::Graphics,
            10,
            "set-window-mode",
            "set-window-mode-desc",
        )
        .keywords(&["fullscreen", "borderless", "window"])
        .apply(Apply::WithRevert(15))
        .since("0.2"),
        D::new(
            "graphics.resolution",
            K::Enum(RESOLUTIONS),
            "1920x1080",
            C::Graphics,
            20,
            "set-resolution",
            "set-resolution-desc",
        )
        .keywords(&["size", "window", "screen"])
        .apply(Apply::WithRevert(15))
        .since("0.2"),
        D::new(
            "graphics.vsync",
            K::Bool,
            "false",
            C::Graphics,
            30,
            "set-vsync",
            "set-vsync-desc",
        )
        .keywords(&["tearing", "frame rate", "fps"])
        .since("0.2"),
        D::new(
            "texture_filtering",
            K::Enum(FILTERING),
            "16",
            C::Graphics,
            40,
            "set-texture-filtering",
            "set-texture-filtering-desc",
        )
        .keywords(&["anisotropic", "mipmap", "sharp"]),
        D::new(
            "look",
            K::Enum(LOOKS),
            "vivid",
            C::Graphics,
            50,
            "set-look",
            "set-look-desc",
        )
        .keywords(&["colour", "color", "tonemapping", "saturation"]),
        D::new(
            "graphics.hdr",
            K::Bool,
            "true",
            C::Graphics,
            52,
            "set-hdr",
            "set-hdr-desc",
        )
        .keywords(&["tonemapping", "bloom", "high dynamic range", "16 bit"])
        .since("0.3"),
        D::new(
            "graphics.default_fov",
            K::Int {
                min: 0,
                max: 120,
                step: 5,
            },
            "0",
            C::Graphics,
            55,
            "set-default-fov",
            "set-default-fov-desc",
        )
        .keywords(&["field of view", "fov", "camera", "driver view", "zoom"])
        .since("0.3"),
        D::new(
            "graphics.view_distance",
            K::Int {
                min: 1,
                max: 8,
                step: 1,
            },
            "4",
            C::Graphics,
            60,
            "set-view-distance",
            "set-view-distance-desc",
        )
        .keywords(&["draw distance", "lod"])
        .since("0.4"),
        D::new(
            "graphics.shadow_distance",
            K::Int {
                min: 50,
                max: 2000,
                step: 50,
            },
            "300",
            C::Graphics,
            61,
            "set-shadow-distance",
            "set-shadow-distance-desc",
        )
        .keywords(&["shadows", "cascade", "sun", "distance"])
        .since("0.5"),
        D::new(
            "graphics.light_distance",
            K::Int {
                min: 50,
                max: 2000,
                step: 50,
            },
            "350",
            C::Graphics,
            62,
            "set-light-distance",
            "set-light-distance-desc",
        )
        .keywords(&["lights", "lamps", "street lights", "night", "distance"])
        .since("0.5"),
        D::new(
            "graphics.detail_distance",
            K::Int {
                min: 25,
                max: 800,
                step: 25,
            },
            "200",
            C::Graphics,
            63,
            "set-detail-distance",
            "set-detail-distance-desc",
        )
        .keywords(&["lod", "detail", "scenery", "objects", "distance"])
        .since("0.5"),
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
            "audio.master",
            K::Int {
                min: 0,
                max: 100,
                step: 5,
            },
            "80",
            C::Audio,
            10,
            "set-audio-master",
            "set-audio-master-desc",
        )
        .keywords(&["volume", "sound"])
        .since("0.4"),
        D::new(
            "audio.engine",
            K::Int {
                min: 0,
                max: 100,
                step: 5,
            },
            "80",
            C::Audio,
            20,
            "set-audio-engine",
            "set-audio-engine-desc",
        )
        .keywords(&["volume", "sound"])
        .since("0.4"),
        D::new(
            "audio.effects",
            K::Int {
                min: 0,
                max: 100,
                step: 5,
            },
            "80",
            C::Audio,
            25,
            "set-audio-effects",
            "set-audio-effects-desc",
        )
        .keywords(&["volume", "sound", "doors", "horn", "switches"])
        .since("0.4"),
        D::new(
            "audio.ambience",
            K::Int {
                min: 0,
                max: 100,
                step: 5,
            },
            "60",
            C::Audio,
            30,
            "set-audio-ambience",
            "set-audio-ambience-desc",
        )
        .keywords(&["volume", "sound"])
        .since("0.4"),
        D::new(
            "audio.interface",
            K::Int {
                min: 0,
                max: 100,
                step: 5,
            },
            "60",
            C::Audio,
            40,
            "set-audio-interface",
            "set-audio-interface-desc",
        )
        .keywords(&["volume", "sound", "clicks"])
        .since("0.4"),
        D::new(
            "controls.rebind",
            K::Action,
            "",
            C::Controls,
            20,
            "set-rebind",
            "set-rebind-desc",
        )
        .keywords(&["keys", "keyboard", "gamepad", "wheel", "binding"])
        .not_yet(&SETTINGS_REBIND),
    ]
}
