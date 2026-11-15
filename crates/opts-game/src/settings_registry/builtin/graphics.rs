//! The Graphics page settings (window, texture filtering, look, water, weather).

use super::*;

mod audio_controls;

mod light_range;
mod water_weather;

use light_range::{detail, shadows};
use water_weather::{weather_strength, world};

/// Part 1: graphics, audio, controls.
pub(super) fn graphics() -> Vec<D> {
    [
        display(),
        distances(),
        shadows(),
        detail(),
        world(),
        weather_strength(),
        audio_controls::audio(),
        audio_controls::controls(),
    ]
    .concat()
}

/// Window, texture filtering, look and HDR.
fn display() -> Vec<D> {
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
    ]
}

/// Field of view and the view distance.
fn distances() -> Vec<D> {
    vec![
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
    ]
}
