//! The look of the god rays: the shaft constants and the [`ShaftStyle`] resource, with its
//! developer override ([`TUNE_ENV`]).

use bevy::prelude::*;

/// Side of the camera-following fog cube in metres: twice the shadow cascades' maximum distance
/// (150 m would be the reach; 200 m keeps the ray steps short, the camera is always inside).
pub const SHAFT_VOLUME_SIDE_M: f32 = 200.0;
/// Baseline haze `density_factor` per metre (`HAZE_BASE`): a clear sky has an extinction of about
/// 1e-4 per metre, far too thin to show a shaft inside the shadow range, so this is an artistic
/// haze ❓.
pub const HAZE_BASE: f32 = 0.0006;
/// Factor from the weather's Koschmieder extinction `3.912 / visibility` (per metre) to the
/// volume's `density_factor`. Bevy's extinction is `density_factor × (absorption + scattering)`
/// = `0.6 ×` with the defaults, so 1.0 makes the haze a little thinner than the distance fog.
pub const FOG_TO_SHAFT: f32 = 1.0;
/// Largest `density_factor`: thicker volumes wash the screen out and end every ray early. The
/// distance fog already is the fog; the haze volume only adds the light shafts and its
/// in-scattering takes the *sun's* colour, so a thick one (0.05) tinted the whole frame yellow at
/// a low sun. At 0.008 (extinction 0.005 per metre, 40 % in 100 m) the shafts of a foggy morning
/// stay and the tint is a hint.
pub const HAZE_MAX: f32 = 0.008;
/// `scattering_asymmetry`: strong forward scattering, shafts show when looking towards the light.
pub const SHAFT_ASYMMETRY: f32 = 0.65;
/// Overall factor of the volume's `light_intensity` (`gain`).
pub const SHAFT_GAIN: f32 = 0.9;
/// `VolumetricFog::ambient_intensity` (`ambient`): 0, the default 0.1 veils the whole screen.
pub const SHAFT_AMBIENT: f32 = 0.0;
/// The volume's `absorption` coefficient (`absorption`).
pub const SHAFT_ABSORPTION: f32 = 0.3;
/// The volume's `scattering` coefficient (`scatter`).
pub const SHAFT_SCATTERING: f32 = 0.3;
/// Name of the developer override variable, see [`ShaftStyle::parse_tune`].
pub const TUNE_ENV: &str = "OPTS_SHAFT_TUNE";
/// Brighter shafts for the moon: its light is about 20 times weaker than the sun's in game units,
/// so with the same intensity its beams would be invisible ❓ (tunable).
pub const MOON_SHAFT_GAIN: f32 = 2.0;
/// Intensity scale without `Hdr` on the camera (the fog adds untonemapped light, colours clip).
pub const LDR_SHAFT_SCALE: f32 = 0.6;
/// `VolumetricFog::jitter` in metres (no effect without temporal anti-aliasing, kept on as in the
/// design).
pub const SHAFT_JITTER: f32 = 0.5;

/// The look of the shafts: the constants above as a resource, overridable for tuning with
/// `OPTS_SHAFT_TUNE="haze=0.001,asym=0.6,gain=1,ambient=0,absorption=0.3,scatter=0.3,moon_gain=4"`
/// (any subset, read once at startup; see `docs/src/reference/developer-switches.md`).
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct ShaftStyle {
    /// Baseline haze `density_factor` per metre (`haze`).
    pub haze: f32,
    /// Henyey-Greenstein asymmetry `g` (`asym`).
    pub asym: f32,
    /// Factor of the volume's `light_intensity` (`gain`).
    pub gain: f32,
    /// `VolumetricFog::ambient_intensity` (`ambient`).
    pub ambient: f32,
    /// Absorption coefficient (`absorption`).
    pub absorption: f32,
    /// Scattering coefficient (`scatter`).
    pub scatter: f32,
    /// Extra gain of the moon's shafts (`moon_gain`).
    pub moon_gain: f32,
}

impl Default for ShaftStyle {
    fn default() -> Self {
        Self {
            haze: HAZE_BASE,
            asym: SHAFT_ASYMMETRY,
            gain: SHAFT_GAIN,
            ambient: SHAFT_AMBIENT,
            absorption: SHAFT_ABSORPTION,
            scatter: SHAFT_SCATTERING,
            moon_gain: MOON_SHAFT_GAIN,
        }
    }
}

impl ShaftStyle {
    /// The defaults with the `key=value` pairs of `text` applied (comma separated, unknown keys
    /// and unparsable or negative values ignored; `asym` is clamped below 1).
    #[must_use]
    pub fn parse_tune(text: &str) -> Self {
        let mut st = Self::default();
        for pair in text.split(',') {
            let Some((k, v)) = pair.split_once('=') else {
                continue;
            };
            let Ok(v) = v.trim().parse::<f32>() else {
                continue;
            };
            if !v.is_finite() || v < 0.0 {
                continue;
            }
            match k.trim() {
                "haze" => st.haze = v,
                "asym" => st.asym = v.min(0.99),
                "gain" => st.gain = v,
                "ambient" => st.ambient = v,
                "absorption" => st.absorption = v,
                "scatter" => st.scatter = v,
                "moon_gain" => st.moon_gain = v,
                _ => {}
            }
        }
        st
    }

    /// The style from [`TUNE_ENV`] (the defaults when unset).
    #[must_use]
    pub fn from_env() -> Self {
        std::env::var(TUNE_ENV).map_or_else(|_| Self::default(), |t| Self::parse_tune(&t))
    }
}
