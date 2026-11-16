//! The colour of the fog, the veil it puts on the sky, and the brightness of the cloud layer.

use super::*;

/// Largest saturation `(max - min) / max` of the colour of a full-thickness fog; thinner fog
/// allows more up to the horizon's own colour in clear air.
pub const FOG_MAX_SATURATION: f32 = 0.2;

/// Fog colour: the sky's horizon colour with most of its chroma taken out, in sRGB components
/// `0..=1`, as the horizon texture holds them.
///
/// Real fog is grey-white: droplets scatter all wavelengths alike, so the hue of a sunset horizon
/// (orange, olive) must not tint it. The share of the chroma that goes is
/// `thick * (0.6 + 0.35 * thick)` with `thick` the fogginess (or half the cover), about 94 % in
/// fog of 100 m, and the saturation is capped at `1 - 0.8 * thick` (0.2 in thick fog). The
/// luminance follows the horizon (so the fog is dark at night and as dark under cloud as the
/// horizon texture is), scaled by 0.95. In clear air the colour is the horizon's.
#[must_use]
pub fn fog_color(horizon: [f32; 3], c: &WeatherConditions) -> [f32; 3] {
    let luma = |v: [f32; 3]| (v[0] * 0.3 + v[1] * 0.59 + v[2] * 0.11).max(0.0);
    let grey = luma(horizon) * 0.95;
    let thick = fogginess(c).max(effective_cover(c) * 0.5).clamp(0.0, 1.0);
    let t = thick * (0.6 + 0.35 * thick);
    let mixed = horizon.map(|v| v + (grey - v) * t);
    let hi = mixed.iter().copied().fold(0.0_f32, f32::max);
    let lo = mixed.iter().copied().fold(f32::MAX, f32::min);
    let cap = 1.0 - (1.0 - FOG_MAX_SATURATION) * thick;
    if hi > 1e-6 && (hi - lo) / hi > cap {
        // Pull towards the colour's own luminance until the saturation is at the cap.
        let mean = luma(mixed);
        let k = (cap * mean / ((hi - lo) - cap * (hi - mean))).clamp(0.0, 1.0);
        return mixed.map(|v| mean + (v - mean) * k);
    }
    mixed
}

/// How far the sky texture is veiled towards the fog colour at height `v` (`0` zenith … `1`
/// horizon), `0..=1`: fog covers all of it, an overcast sky greys it mostly towards the horizon
/// (the cloud layer reaches only so far).
#[must_use]
pub fn sky_veil(c: &WeatherConditions, v: f32) -> f32 {
    sky_veil_clear(c, v, 0.0)
}

/// [`sky_veil`] with the fog lifted for a camera above the ground-fog layer
/// ([`GroundFogView::sky_clear`]): the fog part of the veil falls to nothing at the zenith and to
/// half at the horizon (a hazy rim: `0.5 · (1 − elevation/8°)²` below 8° of elevation, the
/// texture row `v` being `1 − elevation/90°`); the overcast part is unchanged.
#[must_use]
pub fn sky_veil_clear(c: &WeatherConditions, v: f32, sky_clear: f32) -> f32 {
    let overcast = c
        .cloud
        .as_ref()
        .filter(|cl| cl.cover == CloudCover::Overcast)
        .map_or(0.0, |_| 0.45 * (0.3 + 0.7 * v.clamp(0.0, 1.0)));
    let elevation_deg = (1.0 - v.clamp(0.0, 1.0)) * 90.0;
    let rim = 0.5 * (1.0 - elevation_deg / 8.0).clamp(0.0, 1.0).powi(2);
    let lift = 1.0 - sky_clear.clamp(0.0, 1.0) * (1.0 - rim);
    (fogginess(c) * 0.9 * lift).max(overcast)
}

/// Brightness of the unlit cloud layer: dark at night, full at day, a little darker under rain.
#[must_use]
pub fn cloud_tint(c: &WeatherConditions, sun_alt_deg: f32) -> f32 {
    let t = ((sun_alt_deg + 8.0) / 18.0).clamp(0.0, 1.0);
    let day = t * t * (3.0 - 2.0 * t);
    let dim = 1.0 - 0.3 * c.precip_rate - 0.15 * fogginess(c);
    (0.04 + 0.96 * day) * dim
}
