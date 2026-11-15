//! Sun and ambient light and the clear colour, from the sky's light curves and the weather.

use bevy::prelude::*;

use super::*;

/// The sun's colour and strength from the sky's light curve at the sun elevation, scaled by the
/// weather (`opts_bevy::weather::light_scales`).
pub(super) fn sun_light(sky: &Sky, env: &SessionEnvironment, elevation: f32) -> (Color, f32) {
    let a = sky.config.light_at(&sky.config.light_a, elevation);
    let mean = (a[0] + a[1] + a[2]) / 3.0;
    let att = env.light_attenuation().direct;
    (
        Color::srgb(a[0].max(0.001), a[1].max(0.001), a[2].max(0.001)),
        SUN_ILLUMINANCE * mean * att,
    )
}

/// The ambient light from the sky's light curves at the sun elevation, scaled by the weather.
pub(super) fn set_ambient(
    sky: &Sky,
    ambient: &mut GlobalAmbientLight,
    env: &SessionEnvironment,
    (elevation, ambient_scale): (f32, f32),
) {
    let b = sky.config.light_at(&sky.config.light_b, elevation);
    let c = sky.config.light_at(&sky.config.light_c, elevation);
    let amb = [c[0] + b[0] * 0.5, c[1] + b[1] * 0.5, c[2] + b[2] * 0.5];
    let peak = amb.iter().copied().fold(0.001_f32, f32::max);
    ambient.color = Color::srgb(amb[0] / peak, amb[1] / peak, amb[2] / peak);
    ambient.brightness =
        AMBIENT_BRIGHTNESS * ambient_scale * peak.min(1.0) * env.light_attenuation().ambient;
}

/// Writes the clear colour; with HDR what shows the colour after the camera's tonemapping.
pub(super) fn set_clear_colour(
    clear: &mut ClearColor,
    look: Option<&crate::look::ActiveLook>,
    hdr: bool,
    clear_color: [f32; 3],
) {
    let display = Color::srgb(clear_color[0], clear_color[1], clear_color[2]).to_linear();
    clear.0 = if hdr {
        // The HDR camera tonemaps its clear colour too; write what shows `display` after it.
        let tm = look
            .as_ref()
            .map_or_else(Default::default, |l| l.0.params().tonemapping);
        let x = crate::look::hdr_clear_color(tm, [display.red, display.green, display.blue]);
        Color::linear_rgb(x[0], x[1], x[2])
    } else {
        display.into()
    };
}

/// The clear colour fills what the dome does not cover: the fog colour, towards the horizon
/// colour as the camera leaves the ground-fog layer.
pub(super) fn clear_colour_of(
    horizon: [f32; 3],
    veil_color: [f32; 3],
    fog_on: bool,
    fog_clear: u8,
) -> [f32; 3] {
    let lift = f32::from(fog_clear) / 32.0;

    if fog_on {
        [0, 1, 2].map(|c| veil_color[c] + (horizon[c] - veil_color[c]) * lift)
    } else {
        horizon
    }
}
