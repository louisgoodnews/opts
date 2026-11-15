//! The strength of the rays: the haze density from the weather, the light intensity of the volume
//! and the choice of the carrier light.


use crate::light_quality::Shafts;
use super::*;

/// The `density_factor` of the haze for a visibility in metres: the larger of the baseline
/// [`HAZE_BASE`] and the weather's extinction (`β = 3.912 / V`, Koschmieder) times
/// [`FOG_TO_SHAFT`], at most [`HAZE_MAX`]. Fog thickens the haze; clear air keeps the baseline.
#[must_use]
pub fn haze_density(visibility_m: f32, style: &ShaftStyle) -> f32 {
    let beta = 3.912 / visibility_m.max(1.0);
    (beta * FOG_TO_SHAFT).clamp(style.haze, HAZE_MAX.max(style.haze))
}

/// The `light_intensity` of the volume: the weather's shaft attenuation, the moon's gain and the
/// LDR scale.
#[must_use]
pub fn shaft_intensity(
    role: LightRole,
    shafts_attenuation: f32,
    hdr: bool,
    style: &ShaftStyle,
) -> f32 {
    let gain = match role {
        LightRole::Sun => style.gain,
        LightRole::Moon => style.gain * style.moon_gain,
    };
    let ldr = if hdr { 1.0 } else { LDR_SHAFT_SCALE };
    shafts_attenuation.clamp(0.0, 1.0) * gain * ldr
}

/// The index of the light that gets [`VolumetricLight`], if any: the first light with shadows and
/// light, of a role the quality allows, when shafts are on.
#[must_use]
pub fn pick_carrier(
    shafts: Shafts,
    night_shafts: bool,
    lights: impl IntoIterator<Item = (LightRole, bool, f32)>,
) -> Option<usize> {
    if shafts == Shafts::Off {
        return None;
    }
    lights.into_iter().position(|(role, shadows, illuminance)| {
        shadows && illuminance > 0.0 && (role == LightRole::Sun || night_shafts)
    })
}
