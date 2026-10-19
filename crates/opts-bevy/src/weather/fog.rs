//! Fog, sky veil and cloud tint derived from the weather.

use super::*;

/// Visibility above which no distance fog is drawn (metres).
pub const FOG_MAX_VISIBILITY_M: f32 = 15_000.0;

/// How the weather scales the sun's illuminance and the ambient brightness.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LightScales {
    /// Multiplier of the direct sunlight.
    pub sun: f32,
    /// Multiplier of the ambient light.
    pub ambient: f32,
}

/// How foggy the air is, `0` (10 km visibility or more) to `1` (a few metres).
#[must_use]
pub fn fogginess(c: &WeatherConditions) -> f32 {
    (1.0 - c.fog_visibility_m / 10_000.0).clamp(0.0, 1.0)
}

/// The share of the sky that is covered for lighting: the larger of the clouds, the
/// precipitation (rain and snow come out of a covered sky) and the thick fog.
#[must_use]
pub fn effective_cover(c: &WeatherConditions) -> f32 {
    let clouds = c.cloud.as_ref().map_or(0.0, |cl| cl.coverage);
    let precip = if c.precip == PrecipKind::None || c.precip_rate <= 0.0 {
        0.0
    } else {
        0.5 + 0.5 * c.precip_rate
    };
    let fog = fogginess(c) * (0.5 + 0.5 * c.fog_density);
    clouds.max(precip).max(fog).clamp(0.0, 1.0)
}

/// **The one function through which weather scales the sun and the ambient light.** A covered sky
/// (cover 1) gives the sun 0.55 and the ambient light 0.8, a clear one 1 and 1.
#[must_use]
pub fn light_scales(c: &WeatherConditions) -> LightScales {
    light_scales_for_cover(effective_cover(c))
}

/// [`light_scales`] for a given share of covered sky `0..=1` (the rule itself, so the sky
/// description [`crate::environment::WeatherSky`] shares it).
#[must_use]
pub fn light_scales_for_cover(cover: f32) -> LightScales {
    let cover = cover.clamp(0.0, 1.0);
    LightScales {
        sun: 1.0 - 0.45 * cover,
        ambient: 1.0 - 0.2 * cover,
    }
}

/// Distance fog: exponential-squared density per metre, `None` when the air is clear.
///
/// The density makes an object vanish at the visibility: the fog's contrast falls to a threshold
/// at that distance (2 % for a full-thickness layer, up to 10 % for a thin one, so thin fog is
/// lighter at the same visibility).
#[must_use]
pub fn fog_density(c: &WeatherConditions) -> Option<f32> {
    fog_density_for(c, c.fog_visibility_m)
}

/// [`fog_density`] for another visibility than the preset's (the ground-fog layer raises it with
/// the camera's height, see [`ground_fog`]).
#[must_use]
pub fn fog_density_for(c: &WeatherConditions, visibility_m: f32) -> Option<f32> {
    if visibility_m >= FOG_MAX_VISIBILITY_M {
        return None;
    }
    let contrast = 0.02 + 0.08 * (1.0 - c.fog_density);
    Some((-contrast.ln()).sqrt() / visibility_m.max(1.0))
}

/// Presets with a visibility **at or below** this many metres are ground fog (a low layer);
/// anything thinner (dusk fog 500 m, storm 700 m, haze) is fog everywhere, as before. ❓ The
/// corpus has one ground-fog preset (`Bodennebel`, 75 m); `.owt` has no "layer" flag.
pub const FOG_LAYER_MAX_VISIBILITY_M: f32 = 150.0;

/// Layer thickness as a share of the visibility ❓ (`Bodennebel`: 0.4 × 75 m = 30 m).
pub const FOG_LAYER_THICKNESS_PER_VISIBILITY: f32 = 0.4;

/// Thinnest and thickest layer in metres.
pub const FOG_LAYER_THICKNESS_RANGE_M: (f32, f32) = (25.0, 200.0);

/// Below this share of the layer thickness the camera is inside the layer (full fog).
pub const FOG_LAYER_INSIDE: f32 = 0.5;

/// Above this multiple of the thickness the camera is clear of the layer.
pub const FOG_LAYER_ABOVE: f32 = 2.0;

/// Looking up by this many degrees (from above the layer) takes the fog off what is seen.
pub const FOG_LAYER_LOOK_UP_DEG: f32 = 15.0;

/// How much farther the visibility reaches when looking fully up from above the layer.
pub const FOG_LAYER_LOOK_UP_GAIN: f32 = 3.0;

/// The thickness in metres of the ground-fog layer of the conditions, `None` when the preset is
/// not ground fog (visibility above [`FOG_LAYER_MAX_VISIBILITY_M`]).
///
/// The `.owt` second value is a unitless factor `0.5 … 1`, not metres (📖 environment.md), so the
/// thickness is a documented estimate: the visibility times
/// [`FOG_LAYER_THICKNESS_PER_VISIBILITY`], scaled by `0.5 + 0.5 ·` that factor, clamped to
/// [`FOG_LAYER_THICKNESS_RANGE_M`].
#[must_use]
pub fn fog_layer_thickness(c: &WeatherConditions) -> Option<f32> {
    if c.fog_visibility_m > FOG_LAYER_MAX_VISIBILITY_M {
        return None;
    }
    let (lo, hi) = FOG_LAYER_THICKNESS_RANGE_M;
    Some(
        (c.fog_visibility_m * FOG_LAYER_THICKNESS_PER_VISIBILITY * (0.5 + 0.5 * c.fog_density))
            .clamp(lo, hi),
    )
}

/// What the ground-fog layer does to the fog and the sky for the camera's height and pitch.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FogParams {
    /// Visibility in metres to feed the distance fog (the preset's, raised above the layer).
    pub visibility_m: f32,
    /// How far the camera is clear of the layer, `0` (inside) … `1` (above): the share by which
    /// the sky's fog veil is taken away.
    pub sky_clear: f32,
}

fn smoothstep(lo: f32, hi: f32, x: f32) -> f32 {
    let t = ((x - lo) / (hi - lo)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// The fog for a camera `height_m` above the ground, looking `pitch_deg` above the horizon
/// (negative: down), in a fog layer of `thickness_m`.
///
/// Bevy's `DistanceFog` knows only the distance to a fragment. For a ground fragment at distance
/// `d` seen from height `h > T` the ray runs `T·d/h` through the layer, so the layer equals
/// distance fog with the visibility multiplied by `h/T`, whatever the pitch. This is exact for
/// the ground (the exponential-squared shape keeps the near ground lighter than the exact
/// exponential would) and approximate for objects above it. The pitch term: looking up from above
/// the layer the ray never enters it, so the visibility grows by up to
/// `FOG_LAYER_LOOK_UP_GAIN + 1` times. The sky is not distance fog at all; `sky_clear` tells
/// `sky.rs` how much of its veil to take away.
///
/// Inside the layer (`h ≤ 0.5·T`) the result is the preset's own fog; between `0.5·T` and `2·T`
/// the factor `1 … 2` follows a smoothstep, and from `2·T` up it is `h/T`; all continuous and
/// monotonic in `h`.
#[must_use]
pub fn ground_fog(visibility_m: f32, thickness_m: f32, height_m: f32, pitch_deg: f32) -> FogParams {
    let t = thickness_m.max(1.0);
    let h = height_m.max(0.0);
    let above = smoothstep(FOG_LAYER_INSIDE * t, FOG_LAYER_ABOVE * t, h);
    let ratio = if h >= FOG_LAYER_ABOVE * t {
        h / t
    } else {
        1.0 + above
    };
    let up = smoothstep(0.0, FOG_LAYER_LOOK_UP_DEG, pitch_deg);
    let scale = ratio * (1.0 + FOG_LAYER_LOOK_UP_GAIN * above * up);
    FogParams {
        visibility_m: (visibility_m * scale).min(FOG_MAX_VISIBILITY_M - 1.0),
        sky_clear: above,
    }
}

/// Where the camera is relative to the ground-fog layer; written by the game each frame, read by
/// the fog, the sky and `sim.get`.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Default)]
pub struct GroundFogView {
    /// Camera height above the terrain under it, metres.
    pub height_m: f32,
    /// Camera pitch above the horizon, degrees.
    pub pitch_deg: f32,
    /// Layer thickness in metres (`None`: this weather is not ground fog or the effect is off).
    pub layer_m: Option<f32>,
    /// The visibility the distance fog gets now.
    pub effective_visibility_m: f32,
    /// How much of the sky's veil is taken away, `0..=1`.
    pub sky_clear: f32,
}

/// The [`GroundFogView`] for the conditions and a camera pose (`fog_on`: the fog effect is on).
#[must_use]
pub fn ground_fog_view(
    c: &WeatherConditions,
    fog_on: bool,
    height_m: f32,
    pitch_deg: f32,
) -> GroundFogView {
    let layer = fog_on.then(|| fog_layer_thickness(c)).flatten();
    match layer {
        Some(t) => {
            let p = ground_fog(c.fog_visibility_m, t, height_m, pitch_deg);
            GroundFogView {
                height_m,
                pitch_deg,
                layer_m: Some(t),
                effective_visibility_m: p.visibility_m,
                sky_clear: p.sky_clear,
            }
        }
        None => GroundFogView {
            height_m,
            pitch_deg,
            layer_m: None,
            effective_visibility_m: c.fog_visibility_m,
            sky_clear: 0.0,
        },
    }
}

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
