//! The sky texture: the blend of the three textures by the sun and its horizon colour.

use bevy::prelude::*;

use super::*;

/// The colour of the sky at the horizon (the last row of the textures, blended by `weights`); it
/// fills anything the dome does not cover and the fog takes it too.
pub(super) fn horizon_colour(sky: &Sky, weights: [f32; 3]) -> [f32; 3] {
    let h = sky.textures[0].height - 1;
    let mut horizon = [0.0; 3];
    for (ch, out) in horizon.iter_mut().enumerate() {
        for (t, wt) in sky.textures.iter().zip(weights) {
            *out += f32::from(t.rgba[(h * t.width) * 4 + ch]) * wt / 255.0;
        }
    }
    horizon
}

/// Rebuilds the sky texture (the blend of the three by the sun's elevation, veiled by fog) and
/// returns the colour at the horizon and the fog's veil colour.
pub(super) fn refresh_texture(
    sky: &Sky,
    env: &SessionEnvironment,
    (fog_on, fog_clear, elevation): (bool, u8, f32),
    images: &mut Assets<Image>,
) -> ([f32; 3], [f32; 3]) {
    let weights = sky.config.sky_weights(elevation);
    // Colour of the sky at the horizon fills anything the dome does not cover; the fog
    // takes it too, greyed in thick fog.
    let horizon = horizon_colour(sky, weights);
    let conditions = env.conditions();
    let veil_color = opts_bevy::weather::fog_color(horizon, conditions);
    let data = blended_sky(
        sky,
        weights,
        |v| {
            if fog_on {
                opts_bevy::weather::sky_veil_clear(conditions, v, f32::from(fog_clear) / 32.0)
            } else {
                0.0
            }
        },
        veil_color,
    );
    if let Some(mut img) = images.get_mut(&sky.image) {
        img.data = Some(data);
    }
    (horizon, veil_color)
}
